use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
struct Board { cells: Vec<u8>, size: u8 }
#[derive(Serialize)]
struct ResultBody { valid: bool }
async fn validate(Json(board): Json<Board>) -> Json<ResultBody> {
    Json(ResultBody { valid: sudoku_core::validate_board(&board.cells, board.size) })
}
#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, Router::new().route("/validate", post(validate))).await.unwrap();
}
