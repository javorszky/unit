use crate::unitctl::UnitCtl;
use crate::UnitctlError;

pub async fn cmd(
    cli: &UnitCtl,
    filename: &String
) -> Result<(), UnitctlError> {
    println!("hello world from the watch command");

    Ok(())
}
