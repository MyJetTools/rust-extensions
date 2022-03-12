pub trait MyTimerLogger {
    fn write_info(&self, timer_id: String, message: String);
    fn write_error(&self, timer_id: String, message: String);
}
