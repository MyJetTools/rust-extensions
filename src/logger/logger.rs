use crate::StrOrString;

use super::LogEventContextBuilder;

pub trait Logger {
    fn write_info(
        &self,
        process: StrOrString<'static>,
        message: StrOrString<'static>,
        ctx: LogEventContextBuilder,
    );
    fn write_warning(
        &self,
        process: StrOrString<'static>,
        message: StrOrString<'static>,
        ctx: LogEventContextBuilder,
    );
    fn write_error(
        &self,
        process: StrOrString<'static>,
        message: StrOrString<'static>,
        ctx: LogEventContextBuilder,
    );
    fn write_fatal_error(
        &self,
        process: StrOrString<'static>,
        message: StrOrString<'static>,
        ctx: LogEventContextBuilder,
    );

    fn write_debug_info(
        &self,
        process: StrOrString<'static>,
        message: StrOrString<'static>,
        ctx: LogEventContextBuilder,
    );
}
