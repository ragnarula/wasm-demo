use std::error::Error;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::Request;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use wasmtime::Engine;
use wasmtime::Func;
use wasmtime::Instance;
use wasmtime::Module;
use wasmtime::Store;

fn add_internal(a: i32, b: i32) -> i32 {
    a + b
}

struct AppState {
    engine: Engine,
}

fn run_module(engine: &Engine, module: Bytes) -> Result<i32, Box<dyn Error>> {
    let mut store = Store::new(&engine, "program.wasm");

    let module = Module::from_binary(engine, &module.to_vec())?;

    let callback = Func::wrap(&mut store, |a: i32, b: i32| -> i32 { add_internal(a, b) });

    let instance = Instance::new(&mut store, &module, &[callback.into()])?;

    let add = instance.get_typed_func::<(), i32>(&mut store, "do_work")?;

    let result = add.call(&mut store, ())?;
    Ok(result)
}

async fn handle_run(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> (StatusCode, String) {
    println!("Got module, running");
    let engine = &state.engine;

    let bytes = to_bytes(req.into_body(), usize::MAX).await.unwrap();
    let result = run_module(engine, bytes);
    match result {
        Ok(result) => {
            println!("result: {result}");
            (StatusCode::OK, format!("result: {result}"))
        }
        Err(e) => {
            println!("failed to run module: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to run module: {e}"),
            )
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let engine = Engine::default();

    let state = AppState { engine };
    let app = Router::new()
        .route("/run", post(handle_run))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("running on 0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
