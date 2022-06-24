pub trait Logger {
    fn write_info(&self, process: String, message: String, ctx: Option<String>);
    fn write_error(&self, process: String, message: String, ctx: Option<String>);
    fn write_fatal_error(&self, process: String, message: String, ctx: Option<String>);
}
