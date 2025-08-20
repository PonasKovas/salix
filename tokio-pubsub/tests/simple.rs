use std::error::Error;
use tokio::{
	select,
	sync::mpsc::{Sender, channel, error::SendError},
};
use tokio_pubsub::{PubSubMessage, Publisher, PublisherHandle, TopicControl};

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
			r = publisher
				.drive::<(), _, _>(TopicControl {
					on_topic_subscribe: async |topic: &u32| {
						println!("someone subscribed to topic {topic}");
						current_topic = Some(*topic);
						external_source(external_sender.clone());

						Ok(Ok(()))
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
