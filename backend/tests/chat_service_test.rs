use chat::chat_client::ChatClient;
use chat::Empty;

use tonic::Request;

mod chat{
	tonic::include_proto!("chat");
}

fn print_type_of<T>(obj: &T){
	println!("{:?}", std::any::type_name::<T>());
}
#[tokio::test]
async fn test_call(){
	let mut client = ChatClient::connect("http://[::1]:10000").await.unwrap();
	let mut message_stream = client.receive_incoming_messages(Empty{}).await.unwrap().into_inner();

	dbg!("good(((((((((((((((");
	println!("HEY >>>>>>>>>>>>>>>");
	// panic!("sdf");
	while let Some(msg) = message_stream.message().await.unwrap() {
		
		println!(">>>>msg {:?}", msg);
		print_type_of(&msg);
	}
}