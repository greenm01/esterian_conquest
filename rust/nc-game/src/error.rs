use std::error::Error;

pub fn exit_code_for(_err: &(dyn Error + 'static)) -> Option<i32> {
    None
}
