use user::user_client::UserClient;
use user::Empty;
use user::OtpRequest;
use user::otp_request::Id;

use tonic::Request;

mod user{
	tonic::include_proto!("users");
}

fn print_type_of<T>(obj: &T){
	println!("{:?}", std::any::type_name::<T>());
}

use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;

type HmacSha256 = Hmac<Sha256>;

pub fn create_uuid(payload: &String, key: &String) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    
    mac.update(payload.as_bytes());
    
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    
    hex::encode(code_bytes)
}

pub fn verify_uuid(payload: &String, encoded_msg: &String, key: &String) -> bool {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    
    // mac.update(payload.as_bytes());
    
    let Ok(existing_code) = hex::decode(encoded_msg) else {
        return false;
    };

    mac.verify_slice(&existing_code).is_ok()
}

#[test]
fn mainssss() {
    let key = String::from("secret-key");
    let data = String::from("important_payload_data");

    let uuid = create_uuid(&data, &key);
    println!("Generated UUID: {}", uuid);

    let is_valid = verify_uuid(&data, &uuid, &key);
    println!("Is valid: {}", is_valid);
}

// #[tokio::test]
async fn test_call(){
	// let mut client = ChatClient::connect("http://[::1]:10000").await.unwrap();
	let mut client = UserClient::connect("http://[::1]:10000").await.unwrap();
	// client.request_otp(Request::new(OtpRequest{id: Some(Id::Email("abdulkuddusa4@gmail.com".to_owned()))})).await.unwrap();
	// let mut message_stream = client.receive_incoming_messages(Empty{}).await.unwrap().into_inner();

	// dbg!("good(((((((((((((((");
	// println!("HEY >>>>>>>>>>>>>>>");
	// // panic!("sdf");
	// while let Some(msg) = message_stream.message().await.unwrap() {
		
	// 	println!(">>>>msg {:?}", msg);
	// 	print_type_of(&msg);
	// }
}

