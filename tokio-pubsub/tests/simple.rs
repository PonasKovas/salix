use std::{convert::Infallible, error::Error};
use tokio::{
	select,
	sync::mpsc::{Sender, channel, error::SendError},
};
use tokio_pubsub::{EventReactor, PubSubMessage, Publisher, PublisherHandle};

const MESSAGES: &[&str] = &["hello", "second message", "third message!!!"];

#[tokio::test]
async fn main() -> Result<(), Box<dyn Error>> {
	let mut publisher = Publisher::new();

	tokio::spawn(receiver_task(publisher.handle()));

	let (external_sender, mut external_receiver) = channel(1);

	let mut current_topic = None;
	loop {
		select! {
			update = external_receiver.recv() => {
				if let Some(topic) = &current_topic {
					publisher.publish(topic, update.unwrap())?;
				}
			}
			driver = publisher.drive() => {
				struct MyReactor<'a>{
					current_topic: &'a mut Option<u32>,
					external_sender: &'a Sender<String>,
				}
				impl<'a> EventReactor<u32, (), Infallible> for MyReactor<'a> {
					type Error = &'static str;

					async fn on_subscribe(&mut self, topic: &u32) -> Result<Result<(), Infallible>, Self::Error> {
						println!("someone subscribed to topic {topic}");
						*self.current_topic = Some(*topic);
						external_source(self.external_sender.clone());

						Ok(Ok(()))
					}
					async fn on_unsubscribe(&mut self, _topic: &u32) -> Result<(), Self::Error> {
						Err("bye")
					}
				}
				let reactor =MyReactor {
					current_topic: &mut current_topic,
					external_sender: &external_sender
				};

				if driver.finish(reactor).await.is_err() {
					break;
				}
			},
		}
	}

	Ok(())
}

async fn receiver_task(
	publisher_handle: PublisherHandle<u32, String, ()>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let mut subscriber = publisher_handle.subscribe().await?;

	subscriber.add_topic(123).await?;
	for m in MESSAGES {
		let (topic, msg) = subscriber.recv().await?;
		let msg = match msg {
			PubSubMessage::Ok(x) => x,
			PubSubMessage::Lagged(_n) => {
				panic!("this shouldnt lag with only 3 messages...");
			}
		};

		assert_eq!(msg.as_str(), *m);

		println!("Received {msg} from {topic} topic");
	}

	Ok(())
}

fn external_source(sender: Sender<String>) {
	tokio::spawn(async move {
		for m in MESSAGES {
			sender.send(m.to_string()).await?;
		}

		Result::<_, SendError<String>>::Ok(())
	});
}
