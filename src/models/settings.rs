#[derive(Debug, Clone)]
pub struct Setting {
    #[allow(dead_code)]
    pub id: String,
    pub encrypted_global_api_key: String,
}
