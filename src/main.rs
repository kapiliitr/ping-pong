use rand::random;
use std::env;
use std::fmt::{Display, Formatter, Result};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

type RcLock<T> = Arc<Mutex<T>>;
type RecvMsg = RcLock<Receiver<Message>>;
type SendAllMsg = RcLock<Vec<Option<Sender<Message>>>>;

#[derive(Clone, PartialEq)]
enum MessageType {
    Ping,
    Pong,
}

impl Display for MessageType {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}",
            if *self == MessageType::Ping {
                "Ping"
            } else {
                "Pong"
            }
        )
    }
}

struct Message {
    m_type: MessageType,
    sender_id: usize,
}

impl Message {
    fn new(mt: MessageType, si: usize) -> Message {
        Message {
            m_type: mt,
            sender_id: si,
        }
    }
}

struct Server {
    id: usize,
    receiver: RecvMsg,
    senders: SendAllMsg,
}

impl Server {
    fn new(i: usize, r: RecvMsg, sa: SendAllMsg) -> Server {
        Server {
            id: i,
            receiver: r,
            senders: sa,
        }
    }

    fn send(&self, mt: MessageType) {
        let mut remaining_servers = vec![];
        {
            let cur_sends = self.senders.lock().unwrap();
            for i in 0..cur_sends.len() {
                if cur_sends[i].is_some() && i != self.id {
                    remaining_servers.push(i);
                }
            }
        }
        if remaining_servers.len() != 0 {
            let random_server = random::<usize>() % remaining_servers.len();
            self.send_internal(remaining_servers[random_server], mt);
        } else {
            let mut cur_sends = self.senders.lock().unwrap();
            cur_sends[self.id] = None;
        }
    }

    fn send_internal(&self, to: usize, mt: MessageType) {
        let mut cur_sends = self.senders.lock().unwrap();
        if let Some(sndr) = cur_sends[to].as_ref() {
            sndr.send(Message::new(mt.clone(), self.id))
                .expect(format!("Failed to send message from {} to {}", self.id, to).as_str());
        }
        if mt == MessageType::Pong {
            cur_sends[to] = None;
        }
    }

    fn start(&self) {
        let cur_recv = self.receiver.lock().unwrap();
        let mut recv_iter = cur_recv.iter();
        while let Some(msg) = recv_iter.next() {
            println!(
                "Server {0} received message {1} from server {2}",
                self.id, msg.m_type, msg.sender_id
            );
            if msg.m_type == MessageType::Ping {
                self.send_internal(msg.sender_id, MessageType::Pong);
                self.send(msg.m_type);
            }
        }
    }
}

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        panic!("Usage: ping-pong.exe <num of servers> <start server index from 0>");
    }

    let num_servers: usize = args[1].parse().expect("Unable to parse num of servers.");
    let start_server: usize = args[2]
        .parse()
        .expect("Unable to parse starting server index.");

    if start_server >= num_servers {
        panic!(
            "Usage: starting server index should be between 0 and {} inclusive",
            num_servers - 1
        );
    }

    let mut all_servers: Vec<Arc<Server>> = vec![];
    let senders: SendAllMsg = Arc::new(Mutex::new(vec![]));
    for i in 0..num_servers {
        let (s, r) = channel();
        let ss = senders.clone();
        let server = Server::new(i, Arc::new(Mutex::new(r)), ss);
        all_servers.push(Arc::new(server));
        senders
            .lock()
            .expect(format!("Unable to add sender for server {}", i).as_str())
            .push(Some(s));
    }

    let handles: Vec<JoinHandle<_>> = all_servers
        .iter()
        .map(|server| {
            let local_server = server.clone();
            thread::spawn(move || {
                local_server.start();
            })
        })
        .collect();

    all_servers[start_server].send(MessageType::Ping);

    for handle in handles.into_iter() {
        handle.join().expect("Failed to join thread");
    }
}
