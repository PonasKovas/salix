use std::error::Error;
use tokio::{
	select,
	sync::mpsc::{self, channel},
};
use tokio_pubsub::{PubSubMessage, Publisher, PublisherHandle, TopicControl};

#[tokio::test]
async fn main() -> Result<(), Box<dyn Error>> {
	let mut publisher = Publisher::new();

	let handle = publisher.handle();

	handle.subscribe().await?.add_topic(123).await?;

	tokio::spawn(receiver_task(handle));

	let mut external_source = external_source();

	println!("start");
	loop {
		select! {
			external = external_source.recv() => {
				match external {
					Some(external) => publisher.publish(&123, external)?,
					None => break,
				}
			}
			r = publisher.control_step::<(), _, _>(TopicControl{
				on_topic_create: async |topic: &u32| {
					println!("created new topic {topic}");
					Ok(())
				},
				..Default::default()
			}) => r.unwrap(),
		}
	}

	panic!("end");

	Ok(())
}

async fn receiver_task(
	publisher_handle: PublisherHandle<u32, String>,
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
	}
}

fn external_source() -> mpsc::Receiver<String> {
	let (sender, receiver) = channel(1);

	tokio::spawn(async move {
		sender.send("hello".to_string()).await?;
		sender.send("second message".to_string()).await?;
		sender.send("third message!!!".to_string()).await?;

		Result::<_, Box<dyn Error + Send + Sync>>::Ok(())
	});

	receiver
}
