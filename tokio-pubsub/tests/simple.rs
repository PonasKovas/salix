use std::error::Error;
use tokio::{
	select,
	sync::mpsc::{Sender, channel, error::SendError},
};
use tokio_pubsub::{PubSubMessage, Publisher, PublisherHandle, TopicControl};

#[tokio::test]
async fn main() -> Result<(), Box<dyn Error>> {
	let mut publisher = Publisher::new();

	// let mut external_source = external_source(Arc::clone(&activate_external));
	let (external_sender, mut external_receiver) = channel(1);

	tokio::spawn(receiver_task(publisher.handle()));

	let mut current_topic = None;
	loop {
		select! {
			update = external_receiver.recv() => {
				if let Some(topic) = &current_topic {
					publisher.publish(topic, update.unwrap())?;
				}
			}
			r = publisher
				.drive::<(), _, _>(TopicControl {
					on_topic_subscribe: async |topic: &u32| {
						println!("someone subscribed to topic {topic}");
						current_topic = Some(*topic);
						external_source(external_sender.clone());

						Ok(())
					},
					on_topic_unsubscribe: async |_topic: &u32| Err(()),
				}) => {
					if r.is_err() { break }
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
	loop {
		let (topic, msg) = subscriber.recv().await?;
		let msg = match msg {
			PubSubMessage::Ok(x) => x,
			PubSubMessage::Lagged(n) => {
				println!("missed {n} messages");
				continue;
			}
		};

		println!("Received {msg} from {topic} topic");

		if msg.as_str() == "please end this test...." {
			break;
		}
	}

	Ok(())
}

fn external_source(sender: Sender<String>) {
	tokio::spawn(async move {
		sender.send("hello".to_string()).await?;
		sender.send("second message".to_string()).await?;
		sender.send("third message!!!".to_string()).await?;
		sender.send("please end this test....".to_string()).await?;

		Result::<_, SendError<String>>::Ok(())
	});
}
