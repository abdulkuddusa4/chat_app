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
#[tokio::test]
async fn test_call(){
	// let mut client = ChatClient::connect("http://[::1]:10000").await.unwrap();
	let mut client = UserClient::connect("http://[::1]:10000").await.unwrap();
	client.request_otp(Request::new(OtpRequest{id: Some(Id::Email("abdulkuddusa4@gmail.com".to_owned()))})).await.unwrap();
	// let mut message_stream = client.receive_incoming_messages(Empty{}).await.unwrap().into_inner();

	// dbg!("good(((((((((((((((");
	// println!("HEY >>>>>>>>>>>>>>>");
	// // panic!("sdf");
	// while let Some(msg) = message_stream.message().await.unwrap() {
		
	// 	println!(">>>>msg {:?}", msg);
	// 	print_type_of(&msg);
	// }
}

