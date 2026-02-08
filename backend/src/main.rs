#![allow(warnings,unused_variables,dead_code)]

use tonic::{
    transport::Server,
    Request, Response,
    Status, 
};

mod user_service;
mod chat_service;
mod channel_layers;

fn print_type_of<T>(obj: &T){
    println!("{:?}", std::any::type_name::<T>());
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:10000".parse().unwrap();

    print_type_of(&addr);
    println!("{:?}", addr);

    // let (channel_layer, send_handler) = channel_layers::ChannelLayer::<>::new();

    let user_service_obj = user_service::UserService::new();
    let chat_service_obj = chat_service::ChatService::new();

    // let svc = RouteGuideServer::new(route_guide);

    // tokio::spawn(chat_service.run());
    Server::builder()
        .add_service(user_service::UserServer::new(user_service_obj))
        .add_service(chat_service::ChatServer::new(chat_service_obj))
        .serve(addr).await?;


    Ok(())
}