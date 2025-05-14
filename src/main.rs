use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{Json, Router, extract::State, response::IntoResponse, routing::post, serve};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    bids: Arc<RwLock<Vec<Order>>>,
    asks: Arc<RwLock<Vec<Order>>>,
}

#[tokio::main]
async fn main() {
    const TICKER: &str = "GOOGLE";

    let state: AppState = AppState {
        bids: Arc::new(RwLock::new(vec![])),
        asks: Arc::new(RwLock::new(vec![])),
    };
    let users: Vec<User> = vec![
        User {
            id: String::from("1"),
            balances: vec![
                (String::from("GOOGLE"), 10.0),
                (String::from("USD"), 5000.0),
            ]
            .into_iter()
            .collect(),
        },
        User {
            id: String::from("2"),
            balances: vec![
                (String::from("GOOGLE"), 10.0),
                (String::from("USD"), 5000.0),
            ]
            .into_iter()
            .collect(),
        },
    ];

    let router: Router = Router::new()
        .route("/order", post(order))
        .with_state(state.clone());

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    serve(listener, router).await.unwrap();
}

struct User {
    id: String,
    balances: HashMap<String, f64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Side {
    Ask,
    Bid,
}

struct Order {
    user_id: String,
    price: f64,
    quantity: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct OrderDto {
    side: Side,
    price: f64,
    quantity: i32,
    user_id: String,
}

#[axum::debug_handler]
async fn order(
    State(mut state): State<AppState>,
    Json(order_dto): Json<OrderDto>,
) -> impl IntoResponse {
    println!("{:?}", order_dto);
    fill_orders(&mut state, &order_dto);
    Json("Ok")
}

fn fill_orders(state: &mut AppState, order_dto: &OrderDto) {
    let remaining_quantity = order_dto.quantity;

    match order_dto.side {
        Side::Ask => {
            let mut asks = state.asks.write().unwrap();
            for order in asks.iter_mut().rev() {
                if order.price > order_dto.price {
                    break;
                } else {
                    if order.quantity > remaining_quantity {
                        order.quantity -= remaining_quantity;
                        flip_balance();
                    }
                }
            }
        }
        Side::Bid => {}
    }
}

fn flip_balance() {}
