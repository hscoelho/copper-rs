use cu29::prelude::{CuBufferHandle, CuHostMemoryPool};
use std::convert::TryInto;
use std::rc::Rc;
use std::time::Duration;
use std::{io, mem, sync::Arc};
use v4l::buffer::{Metadata, Type};
use v4l::device::Handle;
use v4l::io::traits::{CaptureStream, Stream};
use v4l::memory::Memory;
use v4l::v4l_sys::*;
use v4l::{v4l2, Device};

// A specialized V4L stream that uses Copper Buffers for memory management.
pub struct CuV4LStream {
    v4l_handle: Arc<Handle>,
    v4l_buf_type: Type,
    memory_pool: Rc<CuHostMemoryPool>,
    // Arena matching the vl42 metadata and the Copper Buffers
    arena: Vec<(Metadata, Option<CuBufferHandle>)>,
    arena_last_freed_up_index: usize,
    timeout: Option<i32>,
    active: bool,
}

impl CuV4LStream {
    #[allow(dead_code)]
    pub fn new(dev: &Device, buf_size: usize, buf_type: Type) -> io::Result<Self> {
        CuV4LStream::with_buffers(dev, buf_type, buf_size, 4)
    }

    pub fn with_buffers(
        dev: &Device,
        buf_type: Type,
        buf_size: usize,
        buf_count: u32,
    ) -> io::Result<Self> {
        let memory_pool = CuHostMemoryPool::new(buf_size, buf_count + 1, page_size::get()); // +1 to be able to queue one last buffer before zapping the first
        let mut arena = Vec::new();
        arena.resize(buf_count as usize, (Metadata::default(), None));

        let mut result = CuV4LStream {
            v4l_handle: dev.handle(),
            memory_pool: Rc::new(memory_pool),
            arena,
            arena_last_freed_up_index: 0,
            v4l_buf_type: buf_type,
            active: false,
            timeout: None,
        };
        result.allocate_request_buffers(buf_count)?;
        Ok(result)
    }

    /// Returns the raw device handle
    #[allow(dead_code)]
    pub fn handle(&self) -> Arc<Handle> {
        self.v4l_handle.clone()
    }

    /// Sets a timeout of the v4l file handle.
    pub fn set_timeout(&mut self, duration: Duration) {
        self.timeout = Some(duration.as_millis().try_into().unwrap());
    }

    /// Clears the timeout of the v4l file handle.
    #[allow(dead_code)]
    pub fn clear_timeout(&mut self) {
        self.timeout = None;
    }

    fn buffer_desc(&self) -> v4l2_buffer {
        v4l2_buffer {
            type_: self.v4l_buf_type as u32,
            memory: Memory::UserPtr as u32,
            ..unsafe { mem::zeroed() }
        }
    }

    // From v4l::Arena

    fn requestbuffers_desc(&self) -> v4l2_requestbuffers {
        v4l2_requestbuffers {
            type_: self.v4l_buf_type as u32,
            memory: Memory::UserPtr as u32,
            ..unsafe { mem::zeroed() }
        }
    }

    pub fn allocate_request_buffers(&mut self, count: u32) -> io::Result<u32> {
        // we need to get the maximum buffer size from the format first
        let mut v4l2_fmt = v4l2_format {
            type_: self.v4l_buf_type as u32,
            ..unsafe { mem::zeroed() }
        };
        unsafe {
            v4l2::ioctl(
                self.v4l_handle.fd(),
                v4l2::vidioc::VIDIOC_G_FMT,
                &mut v4l2_fmt as *mut _ as *mut std::os::raw::c_void,
            )?;
        }

        let mut v4l2_reqbufs = v4l2_requestbuffers {
            count,
            ..self.requestbuffers_desc()
        };
        unsafe {
            v4l2::ioctl(
                self.v4l_handle.fd(),
                v4l2::vidioc::VIDIOC_REQBUFS,
                &mut v4l2_reqbufs as *mut _ as *mut std::os::raw::c_void,
            )?;
        }

        Ok(v4l2_reqbufs.count)
    }

    #[allow(dead_code)]
    pub fn release(&mut self) -> io::Result<()> {
        // free all buffers by requesting 0
        let mut v4l2_reqbufs = v4l2_requestbuffers {
            count: 0,
            ..self.requestbuffers_desc()
        };
        unsafe {
            v4l2::ioctl(
                self.v4l_handle.fd(),
                v4l2::vidioc::VIDIOC_REQBUFS,
                &mut v4l2_reqbufs as *mut _ as *mut std::os::raw::c_void,
            )
        }
    }
}

impl Drop for CuV4LStream {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            if let Some(code) = e.raw_os_error() {
                // ENODEV means the file descriptor wrapped in the handle became invalid, most
                // likely because the device was unplugged or the connection (USB, PCI, ..)
                // broke down. Handle this case gracefully by ignoring it.
                if code == 19 {
                    /* ignore */
                    return;
                }
            }

            panic!("{:?}", e)
        }
    }
}

impl Stream for CuV4LStream {
    type Item = CuBufferHandle;

    fn start(&mut self) -> io::Result<()> {
        // Enqueue all buffers once on stream start
        // -1 to leave one buffer unqueued as temporary storage
        for index in 1..self.arena.len() {
            self.queue(index)?;
        }

        self.arena_last_freed_up_index = 0;

        unsafe {
            let mut typ = self.v4l_buf_type as u32;
            v4l2::ioctl(
                self.v4l_handle.fd(),
                v4l2::vidioc::VIDIOC_STREAMON,
                &mut typ as *mut _ as *mut std::os::raw::c_void,
            )?;
        }
        self.active = true;
        Ok(())
    }

    fn stop(&mut self) -> io::Result<()> {
        unsafe {
            let mut typ = self.v4l_buf_type as u32;
            v4l2::ioctl(
                self.v4l_handle.fd(),
                v4l2::vidioc::VIDIOC_STREAMOFF,
                &mut typ as *mut _ as *mut std::os::raw::c_void,
            )?;
        }

        self.active = false;
        Ok(())
    }
}

impl CaptureStream<'_> for CuV4LStream {
    fn queue(&mut self, index: usize) -> io::Result<()> {
        let buffer_handle = CuHostMemoryPool::allocate(&self.memory_pool).ok_or(io::Error::new(
            io::ErrorKind::Other,
            "Failed to allocate buffer",
        ))?;

        let buf: &[u8] = buffer_handle.as_slice();
        let mut v4l2_buf = v4l2_buffer {
            index: index as u32,
            m: v4l2_buffer__bindgen_ty_1 {
                userptr: buf.as_ptr() as std::os::raw::c_ulong,
            },
            length: buf.len() as u32,
            ..self.buffer_desc()
        };
        unsafe {
            v4l2::ioctl(
                self.v4l_handle.fd(),
                v4l2::vidioc::VIDIOC_QBUF,
                &mut v4l2_buf as *mut _ as *mut std::os::raw::c_void,
            )?;
        }
        self.arena[index] = (Metadata::default(), Some(buffer_handle));
        Ok(())
    }

    fn dequeue(&mut self) -> io::Result<usize> {
        let mut v4l2_buf = self.buffer_desc();

        if self
            .v4l_handle
            .poll(libc::POLLIN, self.timeout.unwrap_or(-1))?
            == 0
        {
            // This condition can only happen if there was a timeout.
            // A timeout is only possible if the `timeout` value is non-zero, meaning we should
            // propagate it to the caller.
            return Err(io::Error::new(io::ErrorKind::TimedOut, "VIDIOC_DQBUF"));
        }

        unsafe {
            v4l2::ioctl(
                self.v4l_handle.fd(),
                v4l2::vidioc::VIDIOC_DQBUF,
                &mut v4l2_buf as *mut _ as *mut std::os::raw::c_void,
            )?;
        }
        let index = v4l2_buf.index as usize;
        // println!("dequeue: dequeueing buffer {}", index);
        // println!(" length {}", v4l2_buf.length);
        // println!(" bytesused {}", v4l2_buf.bytesused);
        // println!(" timestamp {:?}", v4l2_buf.timestamp);
        // println!(" sequence {}", v4l2_buf.sequence);
        // println!(" field {}", v4l2_buf.field);

        self.arena[index].0 = Metadata {
            bytesused: v4l2_buf.bytesused,
            flags: v4l2_buf.flags.into(),
            field: v4l2_buf.field,
            timestamp: v4l2_buf.timestamp.into(),
            sequence: v4l2_buf.sequence,
        };

        Ok(index)
    }

    fn next(&mut self) -> io::Result<(&Self::Item, &Metadata)> {
        if !self.active {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Stream is not active. Call start() first.",
            ));
        } else {
            self.queue(self.arena_last_freed_up_index)?;
        };

        let dequeued_index = self.dequeue()?;
        let buffer = self.arena[dequeued_index].1.as_ref().unwrap();
        let meta = &self.arena[dequeued_index].0;
        self.arena_last_freed_up_index = dequeued_index;
        Ok((buffer, meta))
    }
}