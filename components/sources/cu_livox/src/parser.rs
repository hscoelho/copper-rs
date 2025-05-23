use bytemuck::{Pod, Zeroable};
use chrono::{DateTime, Utc};
use cu29::prelude::{CuDuration, CuTime};
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::mem::size_of;
use uom::si::f32::{Length, Ratio};
use uom::si::ratio::ratio;

#[inline(always)]
fn u16_endianness(val: u16) -> u16 {
    if cfg!(target_endian = "little") {
        val
    } else {
        u16::from_le(val)
    }
}

#[inline(always)]
fn u32_endianness(val: u32) -> u32 {
    if cfg!(target_endian = "little") {
        val
    } else {
        u32::from_le(val)
    }
}

#[inline(always)]
fn i32_endianness(val: i32) -> i32 {
    if cfg!(target_endian = "little") {
        val
    } else {
        i32::from_le(val)
    }
}

#[inline(always)]
fn u64_endianness(val: u64) -> u64 {
    if cfg!(target_endian = "little") {
        val
    } else {
        u64::from_le(val)
    }
}

//Tele15
//https://github.com/Livox-SDK/Livox-SDK/wiki/Livox-SDK-Communication-Protocol
// HAP
//https://github.com/Livox-SDK/Livox-SDK2/wiki/Livox-SDK-Communication-Protocol-HAP%28English%29
#[derive(Debug)]
pub enum LivoxError {
    InvalidFrame(String),
    InvalidTimestamp(String),
}

#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct CommandHeader {
    sof: u8,      // Start of frame
    version: u8,  // Protocol version
    length: u16,  // frame length
    cmd_type: u8, // command type
    seq_num: u8,  // Frame Sequence Number                    |
    crc_16: u16,  // Frame Header Checksum
}

impl Debug for CommandHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Magic: {:2X}{:2X}", self.sof, self.version))?;
        f.write_fmt(format_args!(
            "Frame: length {}, cmd_type{:2X}",
            u16_endianness(self.length),
            self.cmd_type
        ))
    }
}

// Command frame format
//
// The Protocol Frame is the smallest unit for control command. Format is as
// follows:
//
// | Field    | Index (byte) | Size (byte) | Description                              |
// | -------- | ------------ | ----------- | ---------------------------------------- |
// | sof      | 0            | 1           | Starting Byte, Fixed to be 0xAA          |
// | version  | 1            | 1           | Protocol Version, 1 for The Current Version |
// | length   | 2            | 2           | Length of The Frame, Max Value: 1400     |
// | cmd_type | 4            | 1           | Command Type: <br>0x00: CMD <br>0x01: ACK <br>0x02: MSG |
// | seq_num  | 5            | 2           | Frame Sequence Number                    |
// | crc_16   | 7            | 2           | Frame Header Checksum                    |
// | data     | 9            | n           | Payload Data                             |
// | crc_32   | 9+n          | 4           | Whole Frame Checksum                     |
//
// Command type description:
// > - **CMD (request)**: Actively send data request - need to return a corresponding ACK;
// > - **ACK (response)**: Response to CMD data;
// > - **MSG (message)**: Actively pushed message data - no need to return response data,
//
// > e.g. broadcast data, pushed message when an error occurs;

#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod, Debug)]
pub struct CommandFrame {
    pub header: CommandHeader,
    pub date: u8,
    pub crc_32: u32,
}

// LiDAR Status Code
//
// LiDAR status_code consists of 32 bits, which has the following meanings:
//
// | Bits     | Data             | Description                                                  |
// | -------- | ---------------- | ------------------------------------------------------------ |
// | Bit0:1   | temp_status      | 0: Temperature in Normal State <br>1: High or Low <br>2: Extremely High or Extremely Low |
// | Bit2:3   | volt_status      | Voltage Status of Internal Module<br> 0: Voltage in Normal State <br>1: High <br>2: Extremely High |
// | Bit4:5   | motor_status     | 0: Motor in Normal State <br>1: Motor in Warning State <br>2: Motor in Error State, Unable to Work |
// | Bit6:7   | dirty_warn       | 0: Not Dirty or Blocked <br>1: Dirty or Blocked              |
// | Bit8     | firmware_status  | 0: Firmware is OK <br>1: Firmware is Abnormal, Need to be Upgraded |
// | Bit9     | pps_status       | 0: No PPS Signal <br>1: PPS Signal is OK                     |
// | Bit10    | device_status    | 0: Normal <br>1: Warning for Approaching the End of Service Life |
// | Bit11    | fan_status       | 0: Fan in Normal State <br>1: Fan in Warning State<br>**Supported devices:** <br/>Mid-40/03.07.0000+ <br/>Horizon/06.04.0000+ <br/>Tele-15/07.03.0000+ |
// | Bit12    | self_heating     | 0: Low Temperature Self Heating On<br>1: Low Temperature Self Heating Off<br>**Supported devices:** <br/>Horizon/06.04.0000+ <br/>Tele-15/07.03.0000+<br/>Mid-70/10.03.0000+<br />Avia/11.06.0000+<br> |
// | Bit13    | ptp_status       | 0: No 1588 Signal <br>1: 1588 Signal is OK                   |
// | Bit14:16 | time_sync_status | 0: System dose not start time synchronization <br>1: Using PTP 1588 synchronization <br>2: Using GPS synchronization <br>3: Using PPS synchronization <br>4: System time synchronization is abnormal (The highest priority synchronization signal is abnormal) |
// | Bit17:29 | RSVD             |                                                              |
// | Bit30:31 | system_status    | 0: Normal <br>1: Warning <br>Any of the following situations will trigger warning: <br>  1.1 `temp_status` is 1;<br>  1.2 `volt_status` is 1;<br>  1.3 `motor_status` is 1;<br>  1.4 `dirty_warn` is 1;<br>  1.5 `device_status` is 1; <br>  1.6 `fan_status` is 1;<br>2: Error <br>Causes the LiDAR to Shut Down and Enter the Error State. <br>Any of the following situations will trigger error: <br>  2.1 `temp_status` is 2;<br>  2.2 `volt_status` is 2;<br>  2.3 `motor_status` is 2;<br>  2.4 `firmware_status` is 1; |

// Timestamp Type:
//
// | Timestamp Type | Source         | Data Type | Description                       |
// | -------------- | -------------- | --------- | --------------------------------- |
// | 0              | No sync source | uint64_t  | Unit: ns                          |
// | 1              | PTP            | uint64_t  | Unit: ns                          |
// | 2              | Reserved       |           |                                   |
// | 3              | GPS            | UTC       | UTC                               |
// | 4              | PPS            | int64_t   | Unit: ns, only supported by LiDAR |

#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct LidarHeader {
    pub version: u8,
    pub slot_id: u8,
    pub lidar_id: u8,
    reserved: u8,
    pub status_code: u32,
    pub timestamp_type: u8,
    pub data_type: u8,
    pub timestamp: u64,
}

impl Debug for crate::parser::LidarHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Version: {:2X} slot_id: {:2X} lidar_id: {:2X} ",
            self.version, self.slot_id, self.lidar_id
        ))?;
        f.write_fmt(format_args!(
            "Status: {}, ts_type{:2X}, data_type{:2X}, timestamp{}",
            u32_endianness(self.status_code),
            self.timestamp_type,
            self.data_type,
            u64_endianness(self.timestamp)
        ))
    }
}

impl LidarHeader {
    pub fn timestamp(&self) -> CuDuration {
        CuDuration(u64_endianness(self.timestamp))
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct PointType2 {
    x: i32,
    y: i32,
    z: i32,
    pub reflectivity: u8,
    pub tag: u8,
}

impl PointType2 {
    pub fn x(&self) -> Length {
        Length::new::<uom::si::length::millimeter>(i32_endianness(self.x) as f32)
    }
    pub fn y(&self) -> Length {
        Length::new::<uom::si::length::millimeter>(i32_endianness(self.y) as f32)
    }
    pub fn z(&self) -> Length {
        Length::new::<uom::si::length::millimeter>(i32_endianness(self.z) as f32)
    }
    pub fn reflectivity(&self) -> Ratio {
        Ratio::new::<ratio>(self.reflectivity as f32 / 255.0)
    }
    pub fn check_invariants(&self) -> Result<(), LivoxError> {
        // TODO: determine the valid range for x,y,z
        Ok(())
    }
}

impl Debug for crate::parser::PointType2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Point: ({}, {}, {}) reflectivity {:2X} tag  {:2X}",
            i32_endianness(self.x),
            i32_endianness(self.y),
            i32_endianness(self.z),
            self.reflectivity,
            self.tag
        ))
    }
}

// **Data Type 2**
//
// Single return cartesian coordinate data format:
//
// | Field        | Offset (byte) | Data Type | Description                              |
// | ------------ | ------------- | --------- | ---------------------------------------- |
// | x            | 0             | int32_t   | X axis, Unit: mm                         |
// | y            | 4             | int32_t   | Y axis, Unit: mm                         |
// | z            | 8             | int32_t   | Z axis, Unit: mm                         |
// | reflectivity | 12            | uint8_t   | Reflectivity                             |
// | tag          | 13            | uint8_t   | For details, see [tag information](#3.4 Tag Information {#tag_info) |
//

// 3.4 Tag Information {#tag_info}
//
// | Bit  | Description                              |
// | ---- | ---------------------------------------- |
// | 7:6  | Reserved                                 |
// | 5:4  | Return Number: <br>00: Return 0<br>01: Return 1<br>02: Return 2<br>03: Return 4 |
// | 3:2  | Point property based on return intensity: <br>00: Normal<br>01: High confidence level of the noise<br>02: Moderate confidence level of the noise<br>03: Reserved |
// | 1:0  | Point property based on spatial position:<br>0: Normal <br>1: High confidence level of the noise<br>2: Moderate confidence level of the noise<br>3: Low confidence level of the noise |
//

// Lidar frame format
// | Field          | Index (byte) | Size (byte) | Description                              |
// | -------------- | ------------ | ----------- | ---------------------------------------- |
// | version        | 0            | 1           | Packet Protocol Version, 5 for the Current Version |
// | slot_id        | 1            | 1           | ID of the Slot Connecting LiDAR Device:<br> If LiDAR connects directly to computer without Hub, default 1;<br> If LiDAR connects to computer through Hub,<br> then the ‘slot_id’ is the corresponding slot number of the hub (range: 1 ~ 9); |
// | LiDAR_id       | 2            | 1           | LiDAR ID:<br> 1: Mid-100 Left / Mid-40 / Tele-15 / Horizon/Mid-70/Avia<br> 2: Mid-100 Middle<br> 3: Mid-100 Right<br>![connection_flow_chart](images/Livox-Mid100.png) |
// | reserved       | 3            | 1           |                                          |
// | status_code    | 4            | 4           | LiDAR Status Indicator Information, For details, see 3.4 |
// | timestamp_type | 8            | 1           | Timestamp Type, For details, see [3.2](#3.2 Time Stamp {#timestamp}) |
// | data_type      | 9            | 1           | Data Type, For details, see [3.3](#3.3 Point Cloud/IMU Data {#data_type}) |
// | timestamp      | 10           | 8           | Nanosecond or UTC Format Timestamp, For details, see 3.2 |
// | data           | 18           | --          | Data information, For details, see [3.3](#3.3 Point Cloud/IMU Data {#data_type}) |

const MAX_POINTS_TYPE2: usize = 96;
pub const DATA_FRAME_TYPE2_SIZE: usize = size_of::<LidarFrame>();

#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod, Debug)]
pub struct LidarFrame {
    pub header: LidarHeader,
    pub points: [PointType2; MAX_POINTS_TYPE2],
}

impl fmt::Display for LivoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LivoxError::InvalidFrame(msg) => write!(f, "Invalid frame: {msg}"),
            LivoxError::InvalidTimestamp(msg) => write!(f, "Invalid timestamp: {msg}"),
        }
    }
}

impl Error for LivoxError {}

pub type RefTime = (DateTime<Utc>, CuTime);

pub fn parse_frame(data: &[u8]) -> Result<&LidarFrame, LivoxError> {
    if data[0] != 0x05
    // Protocol version
    {
        return Err(LivoxError::InvalidFrame(format!(
            "Not a Livox SDK protocol V1 frame: {:2X}",
            data[0],
        )));
    }

    if data.len() < DATA_FRAME_TYPE2_SIZE {
        return Err(LivoxError::InvalidFrame(format!(
            "Frame too short: {} < {}",
            data.len(),
            DATA_FRAME_TYPE2_SIZE
        )));
    }
    if data.len() > DATA_FRAME_TYPE2_SIZE {
        return Err(LivoxError::InvalidFrame(format!(
            "Frame too long: {} > {}",
            data.len(),
            DATA_FRAME_TYPE2_SIZE
        )));
    }
    let packet: &LidarFrame = bytemuck::from_bytes(&data[..DATA_FRAME_TYPE2_SIZE]);
    if packet.header.data_type != 0x02 {
        return Err(LivoxError::InvalidFrame(format!(
            "Point Cloud Data Type 2 expected, received : {:2X}",
            packet.header.data_type
        )));
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use crate::parser::{parse_frame, LidarFrame, RefTime};
    use chrono::prelude::*;
    use cu29::prelude::RobotClock;

    #[test]
    fn test_tele15_packet() {
        let (robot_clock, mock) = RobotClock::mock();
        // push the time by 1s because the first emulated test packet could end up in negative time.
        mock.increment(std::time::Duration::new(1, 0));

        let packet_data: [u8; 1362] = [
            0x05, 0x01, 0x01, 0x00, 0x40, 0x68, 0x00, 0x40, 0x01, 0x02, 0x8B, 0x06, 0xAE, 0xE5,
            0xB4, 0x12, 0xF6, 0x17, 0xF6, 0x5C, 0x00, 0x00, 0x08, 0x08, 0x00, 0x00, 0xAB, 0x05,
            0x00, 0x00, 0x0F, 0x10, 0xFE, 0x5C, 0x00, 0x00, 0x0E, 0x08, 0x00, 0x00, 0x27, 0x05,
            0x00, 0x00, 0x0E, 0x10, 0xE8, 0x5C, 0x00, 0x00, 0x11, 0x08, 0x00, 0x00, 0xA0, 0x04,
            0x00, 0x00, 0x24, 0x10, 0x17, 0x5D, 0x00, 0x00, 0x1A, 0x08, 0x00, 0x00, 0x1B, 0x04,
            0x00, 0x00, 0x0C, 0x10, 0xBD, 0x59, 0x00, 0x00, 0xD4, 0x07, 0x00, 0x00, 0x74, 0x03,
            0x00, 0x00, 0x16, 0x10, 0xDD, 0x58, 0x00, 0x00, 0xC6, 0x07, 0x00, 0x00, 0xED, 0x02,
            0x00, 0x00, 0x09, 0x10, 0xE3, 0x5C, 0x00, 0x00, 0x2D, 0x08, 0x00, 0x00, 0xA7, 0x05,
            0x00, 0x00, 0x0A, 0x10, 0xDE, 0x5C, 0x00, 0x00, 0x31, 0x08, 0x00, 0x00, 0x22, 0x05,
            0x00, 0x00, 0x09, 0x10, 0xE0, 0x5C, 0x00, 0x00, 0x37, 0x08, 0x00, 0x00, 0x9D, 0x04,
            0x00, 0x00, 0x1D, 0x10, 0x32, 0x5C, 0x00, 0x00, 0x2C, 0x08, 0x00, 0x00, 0x0D, 0x04,
            0x00, 0x00, 0x07, 0x10, 0x27, 0x59, 0x00, 0x00, 0xEC, 0x07, 0x00, 0x00, 0x6B, 0x03,
            0x00, 0x00, 0x0E, 0x10, 0x67, 0x54, 0x00, 0x00, 0x84, 0x07, 0x00, 0x00, 0xC4, 0x02,
            0x00, 0x00, 0x1A, 0x10, 0xD5, 0x5C, 0x00, 0x00, 0x52, 0x08, 0x00, 0x00, 0xA3, 0x05,
            0x00, 0x00, 0x0F, 0x10, 0xFA, 0x5C, 0x00, 0x00, 0x5A, 0x08, 0x00, 0x00, 0x1F, 0x05,
            0x00, 0x00, 0x0B, 0x10, 0xAD, 0x5B, 0x00, 0x00, 0x41, 0x08, 0x00, 0x00, 0x89, 0x04,
            0x00, 0x00, 0x04, 0x10, 0x0D, 0x5C, 0x00, 0x00, 0x4E, 0x08, 0x00, 0x00, 0x07, 0x04,
            0x00, 0x00, 0x07, 0x10, 0xDD, 0x59, 0x00, 0x00, 0x21, 0x08, 0x00, 0x00, 0x6E, 0x03,
            0x00, 0x00, 0x13, 0x10, 0x55, 0x54, 0x00, 0x00, 0xA5, 0x07, 0x00, 0x00, 0xBF, 0x02,
            0x00, 0x00, 0x18, 0x10, 0xC4, 0x5B, 0x00, 0x00, 0x5E, 0x08, 0x00, 0x00, 0x8D, 0x05,
            0x00, 0x00, 0x64, 0x10, 0xC1, 0x5A, 0x00, 0x00, 0x4B, 0x08, 0x00, 0x00, 0xFB, 0x04,
            0x00, 0x00, 0x04, 0x10, 0xA4, 0x5B, 0x00, 0x00, 0x65, 0x08, 0x00, 0x00, 0x84, 0x04,
            0x00, 0x00, 0x10, 0x10, 0x2C, 0x5C, 0x00, 0x00, 0x76, 0x08, 0x00, 0x00, 0x04, 0x04,
            0x00, 0x00, 0x14, 0x10, 0xAD, 0x56, 0x00, 0x00, 0xF9, 0x07, 0x00, 0x00, 0x4A, 0x03,
            0x00, 0x00, 0x04, 0x10, 0x37, 0x54, 0x00, 0x00, 0xC4, 0x07, 0x00, 0x00, 0xB9, 0x02,
            0x00, 0x00, 0x18, 0x10, 0xE4, 0x59, 0x00, 0x00, 0x56, 0x08, 0x00, 0x00, 0x6B, 0x05,
            0x00, 0x00, 0x04, 0x10, 0xA4, 0x5B, 0x00, 0x00, 0x84, 0x08, 0x00, 0x00, 0x02, 0x05,
            0x00, 0x00, 0x0C, 0x10, 0xB4, 0x5B, 0x00, 0x00, 0x8A, 0x08, 0x00, 0x00, 0x7F, 0x04,
            0x00, 0x00, 0x13, 0x10, 0xC8, 0x5B, 0x00, 0x00, 0x91, 0x08, 0x00, 0x00, 0xF9, 0x03,
            0x00, 0x00, 0x0D, 0x10, 0xF6, 0x58, 0x00, 0x00, 0x52, 0x08, 0x00, 0x00, 0x5A, 0x03,
            0x00, 0x00, 0x0E, 0x10, 0x6E, 0x52, 0x00, 0x00, 0xBA, 0x07, 0x00, 0x00, 0xA5, 0x02,
            0x00, 0x00, 0x13, 0x10, 0x00, 0x5B, 0x00, 0x00, 0x95, 0x08, 0x00, 0x00, 0x76, 0x05,
            0x00, 0x00, 0x0B, 0x10, 0x21, 0x5B, 0x00, 0x00, 0x9C, 0x08, 0x00, 0x00, 0xF4, 0x04,
            0x00, 0x00, 0x11, 0x10, 0xAB, 0x5A, 0x00, 0x00, 0x95, 0x08, 0x00, 0x00, 0x6B, 0x04,
            0x00, 0x00, 0x12, 0x10, 0xC1, 0x5A, 0x00, 0x00, 0x9B, 0x08, 0x00, 0x00, 0xE7, 0x03,
            0x00, 0x00, 0x0B, 0x10, 0x14, 0x58, 0x00, 0x00, 0x5F, 0x08, 0x00, 0x00, 0x4B, 0x03,
            0x00, 0x00, 0x0E, 0x10, 0x92, 0x52, 0x00, 0x00, 0xDD, 0x07, 0x00, 0x00, 0xA0, 0x02,
            0x00, 0x00, 0x10, 0x10, 0x4A, 0x5A, 0x00, 0x00, 0xA6, 0x08, 0x00, 0x00, 0x64, 0x05,
            0x00, 0x00, 0x0E, 0x10, 0x10, 0x5A, 0x00, 0x00, 0xA4, 0x08, 0x00, 0x00, 0xDE, 0x04,
            0x00, 0x00, 0x12, 0x10, 0xAE, 0x59, 0x00, 0x00, 0x9F, 0x08, 0x00, 0x00, 0x57, 0x04,
            0x00, 0x00, 0x13, 0x10, 0x92, 0x59, 0x00, 0x00, 0xA1, 0x08, 0x00, 0x00, 0xD3, 0x03,
            0x00, 0x00, 0x0A, 0x10, 0x13, 0x57, 0x00, 0x00, 0x68, 0x08, 0x00, 0x00, 0x3A, 0x03,
            0x00, 0x00, 0x0D, 0x10, 0x4B, 0x4F, 0x00, 0x00, 0xAB, 0x07, 0x00, 0x00, 0x7E, 0x02,
            0x00, 0x00, 0x11, 0x10, 0x51, 0x59, 0x00, 0x00, 0xB0, 0x08, 0x00, 0x00, 0x4D, 0x05,
            0x00, 0x00, 0x0D, 0x10, 0x14, 0x59, 0x00, 0x00, 0xAE, 0x08, 0x00, 0x00, 0xC8, 0x04,
            0x00, 0x00, 0x12, 0x10, 0xBE, 0x58, 0x00, 0x00, 0xAA, 0x08, 0x00, 0x00, 0x43, 0x04,
            0x00, 0x00, 0x12, 0x10, 0x89, 0x58, 0x00, 0x00, 0xA9, 0x08, 0x00, 0x00, 0xBF, 0x03,
            0x00, 0x00, 0x0B, 0x10, 0x3C, 0x56, 0x00, 0x00, 0x74, 0x08, 0x00, 0x00, 0x2A, 0x03,
            0x00, 0x00, 0x0E, 0x10, 0xB2, 0x4E, 0x00, 0x00, 0xBA, 0x07, 0x00, 0x00, 0x71, 0x02,
            0x00, 0x00, 0x08, 0x10, 0x5E, 0x58, 0x00, 0x00, 0xBA, 0x08, 0x00, 0x00, 0x35, 0x05,
            0x00, 0x00, 0x0C, 0x10, 0x21, 0x58, 0x00, 0x00, 0xB8, 0x08, 0x00, 0x00, 0xB2, 0x04,
            0x00, 0x00, 0x12, 0x10, 0xD0, 0x57, 0x00, 0x00, 0xB3, 0x08, 0x00, 0x00, 0x2F, 0x04,
            0x00, 0x00, 0x12, 0x10, 0x89, 0x57, 0x00, 0x00, 0xB0, 0x08, 0x00, 0x00, 0xAB, 0x03,
            0x00, 0x00, 0x0E, 0x10, 0x34, 0x55, 0x00, 0x00, 0x79, 0x08, 0x00, 0x00, 0x17, 0x03,
            0x00, 0x00, 0x0E, 0x10, 0x06, 0x4E, 0x00, 0x00, 0xC6, 0x07, 0x00, 0x00, 0x64, 0x02,
            0x00, 0x00, 0x11, 0x10, 0x6C, 0x57, 0x00, 0x00, 0xC2, 0x08, 0x00, 0x00, 0x1E, 0x05,
            0x00, 0x00, 0x0D, 0x10, 0x35, 0x57, 0x00, 0x00, 0xC0, 0x08, 0x00, 0x00, 0x9C, 0x04,
            0x00, 0x00, 0x12, 0x10, 0xEB, 0x56, 0x00, 0x00, 0xBC, 0x08, 0x00, 0x00, 0x1B, 0x04,
            0x00, 0x00, 0x12, 0x10, 0xB3, 0x56, 0x00, 0x00, 0xBA, 0x08, 0x00, 0x00, 0x98, 0x03,
            0x00, 0x00, 0x0D, 0x10, 0x27, 0x54, 0x00, 0x00, 0x7C, 0x08, 0x00, 0x00, 0x04, 0x03,
            0x00, 0x00, 0x0E, 0x10, 0x64, 0x4D, 0x00, 0x00, 0xD1, 0x07, 0x00, 0x00, 0x56, 0x02,
            0x00, 0x00, 0x16, 0x10, 0x81, 0x56, 0x00, 0x00, 0xC9, 0x08, 0x00, 0x00, 0x06, 0x05,
            0x00, 0x00, 0x0E, 0x10, 0x58, 0x56, 0x00, 0x00, 0xC8, 0x08, 0x00, 0x00, 0x87, 0x04,
            0x00, 0x00, 0x12, 0x10, 0x18, 0x56, 0x00, 0x00, 0xC5, 0x08, 0x00, 0x00, 0x07, 0x04,
            0x00, 0x00, 0x10, 0x10, 0xF6, 0x55, 0x00, 0x00, 0xC5, 0x08, 0x00, 0x00, 0x86, 0x03,
            0x00, 0x00, 0x0A, 0x10, 0x81, 0x4A, 0x00, 0x00, 0x9D, 0x07, 0x00, 0x00, 0xA2, 0x02,
            0x00, 0x00, 0x00, 0x10, 0x64, 0x4A, 0x00, 0x00, 0x9D, 0x07, 0x00, 0x00, 0x36, 0x02,
            0x00, 0x00, 0x25, 0x10, 0xA5, 0x55, 0x00, 0x00, 0xD0, 0x08, 0x00, 0x00, 0xEF, 0x04,
            0x00, 0x00, 0x0E, 0x10, 0x80, 0x55, 0x00, 0x00, 0xD0, 0x08, 0x00, 0x00, 0x71, 0x04,
            0x00, 0x00, 0x11, 0x10, 0x4A, 0x55, 0x00, 0x00, 0xCE, 0x08, 0x00, 0x00, 0xF2, 0x03,
            0x00, 0x00, 0x0E, 0x10, 0x36, 0x55, 0x00, 0x00, 0xCF, 0x08, 0x00, 0x00, 0x74, 0x03,
            0x00, 0x00, 0x09, 0x10, 0x8F, 0x4A, 0x00, 0x00, 0xB8, 0x07, 0x00, 0x00, 0x99, 0x02,
            0x00, 0x00, 0x03, 0x10, 0x10, 0x4A, 0x00, 0x00, 0xAE, 0x07, 0x00, 0x00, 0x2A, 0x02,
            0x00, 0x00, 0x23, 0x10, 0xB1, 0x54, 0x00, 0x00, 0xD4, 0x08, 0x00, 0x00, 0xD5, 0x04,
            0x00, 0x00, 0x0E, 0x10, 0xA7, 0x54, 0x00, 0x00, 0xD6, 0x08, 0x00, 0x00, 0x5A, 0x04,
            0x00, 0x00, 0x0E, 0x10, 0x91, 0x54, 0x00, 0x00, 0xD7, 0x08, 0x00, 0x00, 0xDE, 0x03,
            0x00, 0x00, 0x0B, 0x10, 0x7E, 0x54, 0x00, 0x00, 0xD8, 0x08, 0x00, 0x00, 0x61, 0x03,
            0x00, 0x00, 0x09, 0x10, 0x8F, 0x4A, 0x00, 0x00, 0xD1, 0x07, 0x00, 0x00, 0x8F, 0x02,
            0x00, 0x00, 0x07, 0x10, 0x6E, 0x49, 0x00, 0x00, 0xB6, 0x07, 0x00, 0x00, 0x1B, 0x02,
            0x00, 0x00, 0x1D, 0x10, 0xDD, 0x53, 0x00, 0x00, 0xDA, 0x08, 0x00, 0x00, 0xBD, 0x04,
            0x00, 0x00, 0x0E, 0x10, 0xD9, 0x53, 0x00, 0x00, 0xDD, 0x08, 0x00, 0x00, 0x43, 0x04,
            0x00, 0x00, 0x0B, 0x10, 0xC9, 0x53, 0x00, 0x00, 0xDE, 0x08, 0x00, 0x00, 0xC9, 0x03,
            0x00, 0x00, 0x0B, 0x10, 0xDE, 0x53, 0x00, 0x00, 0xE4, 0x08, 0x00, 0x00, 0x4E, 0x03,
            0x00, 0x00, 0x08, 0x10, 0x8D, 0x4B, 0x00, 0x00, 0x05, 0x08, 0x00, 0x00, 0x8C, 0x02,
            0x00, 0x00, 0x09, 0x10, 0x1E, 0x49, 0x00, 0x00, 0xC6, 0x07, 0x00, 0x00, 0x0E, 0x02,
            0x00, 0x00, 0x14, 0x10, 0x24, 0x53, 0x00, 0x00, 0xE1, 0x08, 0x00, 0x00, 0xA6, 0x04,
            0x00, 0x00, 0x0E, 0x10, 0x21, 0x53, 0x00, 0x00, 0xE4, 0x08, 0x00, 0x00, 0x2D, 0x04,
            0x00, 0x00, 0x0A, 0x10, 0x16, 0x53, 0x00, 0x00, 0xE6, 0x08, 0x00, 0x00, 0xB4, 0x03,
            0x00, 0x00, 0x0B, 0x10, 0x32, 0x53, 0x00, 0x00, 0xEC, 0x08, 0x00, 0x00, 0x3B, 0x03,
            0x00, 0x00, 0x08, 0x10, 0x1E, 0x4B, 0x00, 0x00, 0x11, 0x08, 0x00, 0x00, 0x7D, 0x02,
            0x00, 0x00, 0x11, 0x10, 0x81, 0x48, 0x00, 0x00, 0xCC, 0x07, 0x00, 0x00, 0xFE, 0x01,
            0x00, 0x00, 0x16, 0x10, 0x7E, 0x52, 0x00, 0x00, 0xEA, 0x08, 0x00, 0x00, 0x90, 0x04,
            0x00, 0x00, 0x0B, 0x10, 0x6F, 0x52, 0x00, 0x00, 0xEB, 0x08, 0x00, 0x00, 0x17, 0x04,
            0x00, 0x00, 0x0B, 0x10, 0x67, 0x52, 0x00, 0x00, 0xED, 0x08, 0x00, 0x00, 0x9F, 0x03,
            0x00, 0x00, 0x0A, 0x10, 0x8C, 0x52, 0x00, 0x00, 0xF4, 0x08, 0x00, 0x00, 0x27, 0x03,
            0x00, 0x00, 0x08, 0x10, 0xA9, 0x4A, 0x00, 0x00, 0x1C, 0x08, 0x00, 0x00, 0x6D, 0x02,
            0x00, 0x00, 0x11, 0x10, 0x85, 0x48, 0x00, 0x00, 0xE3, 0x07, 0x00, 0x00, 0xF2, 0x01,
            0x00, 0x00, 0x15, 0x10,
        ];

        if packet_data.len() < size_of::<LidarFrame>() {
            panic!("Packet too short: {}", packet_data.len());
        }

        let packet = parse_frame(&packet_data).unwrap();

        let datetime = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
        let _rt: RefTime = (datetime, robot_clock.now());
        let timestamp = packet.header.timestamp;
        println!("Tov: {timestamp}");
    }
}
