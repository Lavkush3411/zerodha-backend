use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, RwLock},
};

use axum::{Json, Router, extract::State, response::IntoResponse, routing::post, serve};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    bids: Arc<RwLock<Vec<Order>>>,
    asks: Arc<RwLock<Vec<Order>>>,
    users: Arc<RwLock<Vec<User>>>,
    ticker:String
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
        Self { bids:Arc::new(RwLock::new(vec![])), asks: Arc::new(RwLock::new(vec![])), users: Arc::new(RwLock::new(users)), ticker: String::from("GOOGLE"), }
    }
}

#[tokio::main]
async fn main() {


    let state: AppState = AppState::default();


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
async fn order(
    State(state): State<AppState>,
    Json(order_dto): Json<OrderDto>,
) -> impl IntoResponse {
    println!("{:?}", order_dto);
    fill_orders(state, &order_dto);
    Json("Ok")
}

fn fill_orders(state:  AppState, order_dto: &OrderDto)->f64 {
    let mut remaining_quantity = order_dto.quantity;
    let mut remove=0;

    match order_dto.side {
        Side::Bid => {
            let mut asks = state.asks.write().unwrap();
            for order in asks.iter_mut().rev() {
                if order.price > order_dto.price {
                    break;
                } else {
                    if order.quantity > remaining_quantity {
                        order.quantity -= remaining_quantity;
                        flip_balance(&order.user_id,&order_dto.user_id, remaining_quantity,order.price,&state);
                        remaining_quantity=0.0;
                    }else {
                        remaining_quantity-=order.quantity;
                        flip_balance(&order.user_id,&order_dto.user_id, order.quantity,order.price,&state);
                        remove+=1
                    }
                }
            }

            for _ in 0..remove{
                asks.pop();
            }
            return remaining_quantity; 
        }
        Side::Ask => {
            let mut bids= state.bids.write().unwrap();

            for order in bids.iter_mut().rev(){
                if order.quantity>remaining_quantity{

                }else{
                    
                }
            }



            return  0.0;

        }
    }
}

fn flip_balance(user1:&String, user2: &String, quantity:f64, price:f64, state:  &AppState) {
    let mut users = state.users.write().unwrap();
    for user in users.iter_mut(){

        // user 1 is seller and user 2 is buyer

        if user.id==*user1{
            match user.balances.entry(state.ticker.clone()) {
                Entry::Occupied(mut entry)=>*entry.get_mut()-=quantity,
                Entry::Vacant(entry)=> println!("{:?} not found",entry)
            }
            match  user.balances.entry("USD".to_string()) {
                Entry::Occupied(mut entry)=>*entry.get_mut()+=price*quantity,
                Entry::Vacant(entry)=>println!("{:?} not found",entry)
            }   
        }
        if user.id==*user2{

            match  user.balances.entry(state.ticker.clone()) {
                Entry::Occupied(mut entry)=>*entry.get_mut()+=quantity,
                Entry::Vacant(entry)=>println!("{:?} not found",entry)
                
            }

            match  user.balances.entry("USD".to_string()) {
                Entry::Occupied(mut entry)=>*entry.get_mut()-=quantity*price,
                Entry::Vacant(entry)=>println!("{:?} not found",entry)
                
            }
            
        }

    }


}
