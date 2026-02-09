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

    OtpVerifyRequest,
    OtpVerifyResponse,
    user_server::{
        UserServer, User
    },
    registration_response::Data,
    otp_verify_response::Res,
};

pub use users::otp_request::Id;
pub use users::otp_request_error::Err as OtpError;

use deadpool_redis::{redis::{cmd, FromRedisValue}, Config, Runtime};

pub mod users{
    tonic::include_proto!("users");
}


pub struct UserService{
    mailing_cred: Credentials,
    redis_pool: deadpool_redis::Pool
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
        let mut redis_conn = self.redis_pool.get().await.unwrap();
        let otp: String = std::iter::repeat_with(fastrand::alphanumeric).take(6).collect();

        match otp_request.id{
            Some(Id::Email(email)) =>{

                let x:() = cmd("SET").arg(&email)
                    .arg( &otp)
                    .arg("EX")
                    .arg(20 as usize)
                    .query_async::<()>(&mut redis_conn)
                    .await.unwrap();

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

    async fn verify_otp(
        &self,
        request: Request<OtpVerifyRequest>
    )
    ->Result<Response<OtpVerifyResponse>, Status>
    {
        let mut redis_conn = self.redis_pool.get().await.unwrap();
        let verify_request = request.into_inner();
        let email_or_phone = &verify_request.email_or_phone;

        let otp:String = match cmd("GET").arg(&email_or_phone).query_async(&mut redis_conn).await{
            Ok(otp)=>otp,
            Err(st)=>return Ok(Response::new(OtpVerifyResponse{res: Some(Res::ErrMsg(st.to_string()))}))
        };

        let otp_verify_msg = if otp==verify_request.otp{
            OtpVerifyResponse{res: Some(Res::Uuid("something".to_string()))}
        }else{
            OtpVerifyResponse{res: Some(Res::ErrMsg("something".to_string()))}
        };

        return Ok(otp_verify_msg);

    }
}


impl UserService{
    pub fn new()->Self{
        let mut cfg = Config::from_url("redis://127.0.0.1:6379");
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();
        Self{
            mailing_cred: Credentials::new("abdulkuddusa4@gmail.com".to_owned(), "lypo whlp okjv ygii".to_owned()),
            redis_pool: pool
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