mod keep;

use std::collections::HashMap;
use std::os::fd::{FromRawFd, RawFd};
use std::sync::{Arc, Mutex};

use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use crate::discovery::{Attachments, OriginRegistry};

pub const ATTACHED_AS: &str = "wifi";

type Links = Arc<Mutex<HashMap<String, ByteChannelHandle>>>;

#[derive(Clone)]
pub struct Ours {
    pub engine: Arc<ReticulumNode>,
    pub origins: Arc<OriginRegistry>,
    pub attachments: Arc<Attachments>,
}

#[must_use]
pub struct GroupHandle {
    tasks: Vec<JoinHandle<()>>,
    links: Links,
    ours: Ours,
}

impl GroupHandle {
    pub fn detach(mut self) {
        self.teardown();
    }

    fn teardown(&mut self) {
        for task in std::mem::take(&mut self.tasks) {
            task.abort();
        }
        let mut links = self.links.lock().unwrap_or_else(|held| held.into_inner());
        for (name, _closes_the_socket_when_dropped) in links.drain() {
            self.ours.attachments.release(&name);
        }
    }
}

impl Drop for GroupHandle {
    fn drop(&mut self) {
        self.teardown();
    }
}

pub fn joined(
    ours: Ours,
    a_socket_connected_to_the_group_owner: RawFd,
) -> std::io::Result<GroupHandle> {
    let adopted =
        unsafe { std::net::TcpStream::from_raw_fd(a_socket_connected_to_the_group_owner) };
    adopted.set_nonblocking(true)?;
    let links: Links = Links::default();
    keep::this_peer(&ours, &links, TcpStream::from_std(adopted)?);
    Ok(GroupHandle {
        tasks: Vec::new(),
        links,
        ours,
    })
}

pub fn hosting(
    ours: Ours,
    a_socket_listening_where_phones_join: RawFd,
) -> std::io::Result<GroupHandle> {
    let adopted =
        unsafe { std::net::TcpListener::from_raw_fd(a_socket_listening_where_phones_join) };
    adopted.set_nonblocking(true)?;
    let listener = TcpListener::from_std(adopted)?;
    let links: Links = Links::default();
    let tasks = vec![tokio::spawn(keep::whoever_joins(
        ours.clone(),
        links.clone(),
        listener,
    ))];
    Ok(GroupHandle { tasks, links, ours })
}
