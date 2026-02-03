#![allow(warnings,unused_variables,dead_code)]

use tonic::{
    transport::Server,
    Request, Response,
    Status, 
};
use tokio_stream::{wrappers::ReceiverStream, Stream};

pub use chat::{
    IncomingMessage,
    Empty,
    chat_server::{
        ChatServer, Chat
    },
};


pub mod chat{
    tonic::include_proto!("chat");
}


#[derive(Default)]
pub struct ChatService;


#[tonic::async_trait]
impl Chat for ChatService{
    type ReceiveIncomingMessagesStream =
    ReceiverStream<Result<IncomingMessage, Status>>;
    async fn receive_incoming_messages(
        &self,
        skip: Request<Empty>
    )
    ->
    Result<Response<Self::ReceiveIncomingMessagesStream>, Status>
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(5);
        tokio::spawn(async move {
                loop{
                    let mut st = String::new();

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    sender.send(Ok(IncomingMessage{text: String::from("my msg")})).await;
                }
            });
        Ok(Response::new(ReceiverStream::new(receiver)))
    }
}
