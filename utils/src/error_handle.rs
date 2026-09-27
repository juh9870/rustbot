use rootcause::Report;

pub fn transform_error(err: Report) -> Report {
    rootcause::report!("{}", err)
}
