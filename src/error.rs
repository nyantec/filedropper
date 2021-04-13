use custom_error::custom_error;

custom_error! {pub Error
    IO { source: std::io::Error } = "{}",
    HTTP { source: hyper::Error } = "{}",
}

pub type Result<T> = std::result::Result<T, Error>;
