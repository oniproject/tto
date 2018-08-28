use serde_json::to_writer;
use std::{
    thread,
    io::Write,
    sync::{
        Mutex,
        mpsc::{channel, Receiver, Sender},
        atomic::{AtomicUsize, Ordering},
    },
};

use {
    local::{Local, LOCAL},
    Args,
    Base,
    Event,
};

lazy_static! {
    pub static ref GLOBAL: Mutex<Global> = Mutex::new(Global::new());
}

pub struct Global {
    tx: Sender<Event>,
    rx: Receiver<Event>,
    threads: Vec<(String, Option<usize>)>,
    skip: AtomicUsize,
}

impl Global {
    fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            tx, rx,
            threads: Vec::new(),
            skip: AtomicUsize::new(0),
        }
    }

    pub fn create_sender(&self) -> Sender<Event> {
        self.tx.clone()
    }

    pub fn register_thread(&mut self, sort_index: Option<usize>) {
        let id = self.threads.len();
        let current = thread::current();
        let tid = current.id();
        let name = current.name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("<unnamed-{}-{:?}>", id, tid));

        self.register_thread_with_name(name, sort_index);
    }

    pub fn register_thread_with_name(&mut self, name: String, sort_index: Option<usize>) {
        let id = self.threads.len();
        self.threads.push((name, sort_index));

        LOCAL.with(|local| {
            assert!(local.borrow().is_none());
            *local.borrow_mut() = Some(Local::new(id, self.tx.clone()));
        });
    }

    pub fn write_profile<W: Write>(&self, mut w: W) {
        // Stop reading samples that are written after
        // write_profile_json() is called.

        self.tx.send(Event::Barrier).ok();

        let skip = self.skip.swap(self.threads.len(), Ordering::Relaxed);

        let names = self.threads.iter()
            .skip(skip)
            .cloned()
            .enumerate()
            .map(|(i, th)| Event::Meta {
                base: Base {
                    name: "thread_name".into(),
                    tid: i,
                    pid: 0,
                    cat: None,
                    args: Args::Name { name: th.0.into() },
                    cname: None,
                },
            });

        let sort_index = self.threads.iter()
            .skip(skip)
            .cloned()
            .enumerate()
            .filter_map(|(i, th)| th.1.map(|idx| (i, idx)))
            .map(|(i, sort_index)| Event::Meta {
                base: Base {
                    name: "thread_sort_index".into(),
                    tid: i,
                    pid: 0,
                    cat: None,
                    args: Args::SortIndex { sort_index },
                    cname: None,
                },
            });

        let iter = names
            .chain(sort_index)
            .chain(self.rx.try_iter().take_while(|e| !e.is_barrier()));

        for e in iter {
            to_writer(&mut w, &e).unwrap();
            w.write(b",\n").unwrap();
        }

        /*
        while let Ok(event) = self.samples.1.try_recv() {
            if event.t0 > start_time {
                break;
            }

            let t0 = event.t0 / 1000;
            let t1 = event.t1 / 1000;

            /*
            data.push(json!({
                "pid": 0,
                "tid": sample.tid.0,
                "name": sample.name,
                "ph": "B",
                "ts": t0,
                "args": json!({
                    "module": sample.location.module,
                    "file": sample.location.file,
                    "line": sample.location.line,
                })
            }));

            data.push(json!({
                "pid": 0,
                "tid": sample.tid.0,
                "ph": "E",
                "ts": t1
            }));
            */
        }
        */
    }
}
