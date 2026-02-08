use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct ChannelLayer<T>{
	receiver: tokio::sync::mpsc::Receiver<Command<T>>,
	subscribers: HashMap<String, tokio::sync::mpsc::Sender<T>>
}

pub enum Command<T>{
	Subscribe((String, tokio::sync::mpsc::Sender<T>)),
	Message((String, T))
}

impl <T>ChannelLayer<T>{
	pub fn new()->(Self, tokio::sync::mpsc::Sender<Command<T>>){
		let subscribers = HashMap::<String, tokio::sync::mpsc::Sender<T>>::new();
		let (sender, receiver) = tokio::sync::mpsc::channel::<Command<T>>(128);
		(
			Self{
					receiver,
					subscribers
			},
			sender
		)
	}

	pub async fn handover_to_runtime(mut self){
		loop {
			match self.receiver.recv().await{
				Some(Command::Subscribe((addr, sender))) => {self.subscribers.insert(addr, sender);},
				Some(Command::Message((addr, message))) => {
					match self.subscribers.get(&addr){
						Some(msg_receiver) => {msg_receiver.send(message).await;}
						None => {dbg!("receiver is not active:");}
					}
				},
				_ => {
					dbg!("sender droped..");
					dbg!("panicking");
					break;
				}
			}
		}
		dbg!("channel droped.");
		dbg!("chat service unstable without channel");
		panic!("shutting down chat service");
	}
}