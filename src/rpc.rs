use crate::flpcp::QueryRes;
use uuid::Uuid;

#[tarpc::service]
pub trait Agg {
    async fn check_proof(uuid: Uuid, res: QueryRes) -> bool;
}
