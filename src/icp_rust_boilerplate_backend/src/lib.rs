#[ic_cdk::query]
fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Message {
    id: u64,
    jenis: String,
    harga: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Message {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Message {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Message, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct MessagePayload {
    jenis: String,
    harga: u64,
}

#[ic_cdk::query]
fn get_message(id: u64) -> Result<Message, Error> {
    match _get_message(&id) {
        Some(message) => Ok(message),
        None => Err(Error::NotFound {
            msg: format!("a message with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn addPesanan(message: MessagePayload) -> Option<Message> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let message = Message {
        id,
        jenis: message.jenis,
        harga: message.harga,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&message);
    Some(message)
}

#[ic_cdk::update]
fn updatePesanan(id: u64, payload: MessagePayload) -> Result<Message, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut message) => {
            message.jenis = payload.jenis;
            message.harga = payload.harga;
            message.updated_at = Some(time());
            do_insert(&message);
            Ok(message)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update a message with id={}. message not found",
                id
            ),
        }),
    }
}

// helper method to perform insert.
fn do_insert(message: &Message) {
    STORAGE.with(|service| service.borrow_mut().insert(message.id, message.clone()));
}

#[ic_cdk::update]
fn deletePesanan(id: u64) -> Result<Message, Error> {
    let deleted_message = match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(message) => message,
        None => return Err(Error::NotFound {
            msg: format!(
                "couldn't delete a message with id={}. message not found.",
                id
            ),
        }),
    };

    // Reset ID counter
    ID_COUNTER.with(|counter| {
        counter.borrow_mut().set(deleted_message.id);
    });

    Ok(deleted_message)
}


#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// a helper method to get a message by id. used in get_message/update_message
fn _get_message(id: &u64) -> Option<Message> {
    STORAGE.with(|service| service.borrow().get(id))
}

#[ic_cdk::query]
fn getTotalHargaPesanan() -> u64 {
    let mut total_harga = 0;

    // Iterate through all messages in STORAGE
    STORAGE.with(|service| {
        for (_, message) in service.borrow().iter() {
            total_harga += message.harga;
        }
    });

    total_harga
}

#[ic_cdk::query]
fn getAvgTotalHarga() -> Option<f64> {
    let mut total_harga = 0;
    let mut count = 0;

    // Iterate through all messages in STORAGE
    STORAGE.with(|service| {
        for (_, message) in service.borrow().iter() {
            total_harga += message.harga;
            count += 1;
        }
    });

    // Calculate average if there are messages, else return None
    if count > 0 {
        Some(total_harga as f64 / count as f64)
    } else {
        None
    }
}



// need this to generate candid
ic_cdk::export_candid!();