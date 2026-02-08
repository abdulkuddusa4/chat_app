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

use crate::channel_layers::{Command, ChannelLayer};


pub mod chat{
    tonic::include_proto!("chat");
}


pub struct ChatService{
    channel_handler: tokio::sync::mpsc::Sender<Command<Result<IncomingMessage, Status>>>
}


#[tonic::async_trait]
impl Chat for ChatService{
    type ReceiveIncomingMessagesStream =
    ReceiverStream<Result<IncomingMessage, Status>>;

    async fn receive_incoming_messages(
        &self,
        request: Request<Empty>
    )
    ->
    Result<Response<Self::ReceiveIncomingMessagesStream>, Status>
    {
        let user_id = match request.metadata().get("authorization"){
            Some(auth_payload) =>{

            },
            None => {
                return Err(Status::unauthenticated("invalid or missing tokens"));
            }
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(5);
        self.channel_handler.send(Command::Subscribe(("id".to_string(), sender))).await;

        Ok(Response::new(ReceiverStream::new(receiver)))
    }


    async fn send_message(
        &self,
        request: Request<IncomingMessage>
    )
    ->
    Result<Response<Empty>, Status>
    {   
        let message = request.into_inner();
        let cmd = Command::Message((message.to_addr.clone(), Ok(message)));
        self.channel_handler.send(cmd).await;

        Ok(Response::new(Empty{}))
    }
}


impl ChatService{
    pub fn new()->Self{
        let (mut message_channel, channel_handler) = ChannelLayer::new();
        tokio::spawn(message_channel.handover_to_runtime());
        return ChatService{channel_handler};
    }
}