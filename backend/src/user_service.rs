#![allow(warnings,unused_variables,dead_code)]

use fastrand;

use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, AsyncSmtpTransport, Transport};
use lettre::{AsyncTransport, Tokio1Executor};
use tonic::{
    Request, Response,
    Status, 
};
use tokio_stream::{wrappers::ReceiverStream, Stream};


pub use users::{
    RegistrationRequest,
    RegistrationResponse,
    OtpRequest,
    OtpRequestError,
    Empty,
    Token,
    user_server::{
        UserServer, User
    },
    registration_response::Data,
};

pub use users::otp_request::Id;
pub use users::otp_request_error::Err as OtpError;



pub mod users{
    tonic::include_proto!("users");
}


pub struct UserService{
    mailing_cred: Credentials,
}


#[tonic::async_trait]
impl User for UserService{
    async fn request_otp(
        &self,
        request: Request<OtpRequest>
    )
    -> Result<Response<OtpRequestError>, Status>
    {
        let otp_request = request.into_inner();

        match otp_request.id{
            Some(Id::Email(email)) =>{
                let otp: String = std::iter::repeat_with(fastrand::alphanumeric).take(6).collect();
                match self.send_mail(
                    &"Nam".to_owned(),
                    &email,
                    &"Subject".to_owned(),
                    &format!("here is your verification code: {}", otp)
                ).await{
                    Ok(())=>return Ok(Response::new(OtpRequestError{err: None})),
                    Err(st)=>return Ok(Response::new(OtpRequestError{err: Some(OtpError::Email(st))}))  
                }
            },
            Some(Id::Phone(phone)) =>todo!(),
            None => todo!()
        }
        Ok(Response::new(OtpRequestError{err: None}))
    }
}


impl UserService{
    pub fn new()->Self{
        Self{
            mailing_cred: Credentials::new("abdulkuddusa4@gmail.com".to_owned(), "lypo whlp okjv ygii".to_owned())
        }
    }

    pub async fn send_mail(
        &self,
        name: &String, email: &String,
        subject: &String, body: &String
    )->Result<(), String>{
        let email = Message::builder()
            .from(Mailbox::new(Some("NoBody".to_owned()), "nobody@domain.tld".parse().unwrap()))
            .to(Mailbox::new(
                Some(name.to_owned()), 
                match email.parse(){
                    Ok(x)=>x,
                    Err(err)=>return Err(format!("error parsing email: {:?}", err))
                }
            ))
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_owned())
            .unwrap();

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
            .unwrap()
            .credentials(self.mailing_cred.clone())
            .build();

        match mailer.send(email).await {
            Ok(_) => println!("Email sent successfully!"),
            Err(e) => panic!("Could not send email: {e:?}"),
        }
        Ok(())
    }
}