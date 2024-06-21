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
struct Service {
    id: u64,
    customer_name: String,
    device_model: String,
    issue_description: String,
    status: String,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Storable for Service {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Service {
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

    static STORAGE: RefCell<StableBTreeMap<u64, Service, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct ServicePayload {
    customer_name: String,
    device_model: String,
    issue_description: String,
    status: String,
}

#[ic_cdk::query]
fn get_service(id: u64) -> Result<Service, Error> {
    match _get_service(&id) {
        Some(service) => Ok(service),
        None => Err(Error::NotFound {
            msg: format!("a service with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_service(service: ServicePayload) -> Option<Service> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let service = Service {
        id,
        customer_name: service.customer_name,
        device_model: service.device_model,
        issue_description: service.issue_description,
        status: service.status,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&service);
    Some(service)
}

#[ic_cdk::update]
fn update_service(id: u64, payload: ServicePayload) -> Result<Service, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut service) => {
            service.customer_name = payload.customer_name;
            service.device_model = payload.device_model;
            service.issue_description = payload.issue_description;
            service.status = payload.status;
            service.updated_at = Some(time());
            do_insert(&service);
            Ok(service)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update a service with id={}. service not found",
                id
            ),
        }),
    }
}

fn do_insert(service: &Service) {
    STORAGE.with(|service_storage| service_storage.borrow_mut().insert(service.id, service.clone()));
}

#[ic_cdk::update]
fn delete_service(id: u64) -> Result<Service, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(service) => Ok(service),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete a service with id={}. service not found.",
                id
            ),
        }),
    }
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

fn _get_service(id: &u64) -> Option<Service> {
    STORAGE.with(|service| service.borrow().get(id))
}

ic_cdk::export_candid!();
