use cu29_derive::gen_cumsgs;
use cu29_export::run_cli;

gen_cumsgs!("copperconfig.ron");

fn main() {
    run_cli::<CuMsgs>().expect("Failed to run the export CLI");
}
