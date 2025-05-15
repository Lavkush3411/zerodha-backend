use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, RwLock},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    serve,
};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    bids: Arc<RwLock<Vec<Order>>>,
    asks: Arc<RwLock<Vec<Order>>>,
    users: Arc<RwLock<Vec<User>>>,
    ticker: String,
}

impl Default for AppState {
    fn default() -> Self {
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
        Self {
            bids: Arc::new(RwLock::new(vec![])),
            asks: Arc::new(RwLock::new(vec![])),
            users: Arc::new(RwLock::new(users)),
            ticker: String::from("GOOGLE"),
        }
    }
}

#[tokio::main]
async fn main() {
    let state: AppState = AppState::default();

    let router: Router = Router::new()
        .route("/order", post(handle_order))
        .route("/balance/{id}", get(get_balance))
        .with_state(state.clone());

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    serve(listener, router).await.unwrap();
}

#[derive(Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize, Debug, Clone)]

struct Order {
    user_id: String,
    price: OrderedFloat<f64>,
    quantity: f64,
}

#[derive(Serialize, Deserialize, Debug)]
struct OrderDto {
    side: Side,
    price: f64,
    quantity: f64,
    user_id: String,
}

#[axum::debug_handler]
async fn get_balance(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let users = state.users.read().unwrap();

    for user in users.iter() {
        if user.id == id {
            return Json(user).into_response();
        }
    }

    return StatusCode::NOT_FOUND.into_response();
}

#[axum::debug_handler]
async fn handle_order(
    State(state): State<AppState>,
    Json(order_dto): Json<OrderDto>,
) -> impl IntoResponse {
    println!("{:?}", order_dto);
    let remaining_to_fill = fill_orders(&state, &order_dto);
    let users = state.users.read().unwrap();
    let current_user_id = order_dto.user_id;
    let price = order_dto.price;

    if remaining_to_fill == 0.0 {
        return Json(
            json!({"unfilled":remaining_to_fill,"filled":order_dto.quantity,"users":*users}),
        )
        .into_response();
    };
    match order_dto.side {
        Side::Bid => {
            let mut bids = state.bids.write().unwrap();
            let order = Order {
                user_id: current_user_id,
                price: OrderedFloat(price),
                quantity: remaining_to_fill,
            };
            bids.push(order);
            bids.sort_by_key(|o| o.price.clone());
        }
        Side::Ask => {
            let mut asks = state.asks.write().unwrap();
            let order = Order {
                user_id: current_user_id,
                price: OrderedFloat(price),
                quantity: remaining_to_fill,
            };
            asks.push(order);
            asks.sort_by_key(|o| o.price.clone());
        }
    };

    return Json(
        json!({"unfilled":remaining_to_fill,"filled":remaining_to_fill-order_dto.quantity, "users":*users}),
    )
    .into_response();
}

fn fill_orders(state: &AppState, order_dto: &OrderDto) -> f64 {
    let mut asked_quantity = order_dto.quantity;
    let mut remove = 0;

    match order_dto.side {
        Side::Bid => {
            let mut asks = state.asks.write().unwrap();
            for order in asks.iter_mut().rev() {
                if order.price > OrderedFloat(order_dto.price) {
                    break;
                } else {
                    if order.quantity > asked_quantity {
                        order.quantity -= asked_quantity;
                        flip_balance(
                            &order.user_id,
                            &order_dto.user_id,
                            asked_quantity,
                            order.price,
                            &state,
                        );
                        asked_quantity = 0.0;
                    } else {
                        asked_quantity -= order.quantity;
                        flip_balance(
                            &order.user_id,
                            &order_dto.user_id,
                            order.quantity,
                            order.price,
                            &state,
                        );
                        remove += 1
                    }
                }
            }

            for _ in 0..remove {
                asks.pop();
            }
            return asked_quantity;
        }
        // ask is sellers ask for a stock
        Side::Ask => {
            let mut bids = state.bids.write().unwrap();
            let mut remove = 0;

            for order in bids.iter_mut().rev() {
                if order.quantity > asked_quantity {
                    order.quantity -= asked_quantity;
                    flip_balance(
                        &order_dto.user_id,
                        &order.user_id,
                        order_dto.quantity,
                        OrderedFloat(order_dto.price),
                        &state,
                    );
                    asked_quantity = 0.0;
                } else {
                    asked_quantity -= order.quantity;
                    flip_balance(
                        &order_dto.user_id,
                        &order.user_id,
                        order.quantity,
                        OrderedFloat(order_dto.price),
                        &state,
                    );
                    remove += 1;
                }
            }

            for _ in 0..remove {
                bids.pop();
            }

            return asked_quantity;
        }
    }
}

fn flip_balance(
    user1: &String,
    user2: &String,
    quantity: f64,
    price: OrderedFloat<f64>,
    state: &AppState,
) {
    let mut users = state.users.write().unwrap();
    for user in users.iter_mut() {
        // user 1 is seller and user 2 is buyer

        if user.id == *user1 {
            match user.balances.entry(state.ticker.clone()) {
                Entry::Occupied(mut entry) => *entry.get_mut() -= quantity,
                Entry::Vacant(entry) => println!("{:?} not found", entry),
            }
            match user.balances.entry("USD".to_string()) {
                Entry::Occupied(mut entry) => *entry.get_mut() += *price * quantity,
                Entry::Vacant(entry) => println!("{:?} not found", entry),
            }
        }
        if user.id == *user2 {
            match user.balances.entry(state.ticker.clone()) {
                Entry::Occupied(mut entry) => *entry.get_mut() += quantity,
                Entry::Vacant(entry) => println!("{:?} not found", entry),
            }

            match user.balances.entry("USD".to_string()) {
                Entry::Occupied(mut entry) => *entry.get_mut() -= *price * quantity,
                Entry::Vacant(entry) => println!("{:?} not found", entry),
            }
        }
    }
}
