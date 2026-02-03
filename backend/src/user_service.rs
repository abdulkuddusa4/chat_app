#![allow(warnings,unused_variables,dead_code)]

use tonic::{
    Request, Response,
    Status, 
};
use tokio_stream::{wrappers::ReceiverStream, Stream};


pub use users::{
    RegistrationRequest,
    RegistrationResponse,
    Token,
    user_server::{
        UserServer, User
    },
    registration_response::Data,
};



pub mod users{
    tonic::include_proto!("users");
}



#[derive(Default)]
pub struct UserService;

#[tonic::async_trait]
impl User for UserService{
    async fn register_user(
        &self,
        request: Request<RegistrationRequest>
    )
    -> Result<Response<RegistrationResponse>, Status>
    {
        Ok(Response::new(RegistrationResponse{
            data:Some(Data::Token(Token{
                access: (&[2,3]).to_vec(),
                ttl_seconds: 5
            }))
        }))
    }
}
