use lazy_static::lazy_static;
use tokio::runtime::Builder;

lazy_static! {
    pub static ref CONTEXT: Context = {
        let tokio_runtime = Builder::new_multi_thread()
            .worker_threads(4)
            .enable_time()
            .enable_io()
            .build();

        Context {
            tokio_runtime: tokio_runtime.expect("Init tokio runtime failed")
        }
    };
}

pub struct Context {
    pub tokio_runtime: tokio::runtime::Runtime,
}
