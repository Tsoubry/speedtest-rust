use serde::Deserialize;

/*ip info country asn database model*/

#[derive(Clone, PartialEq, Eq, Hash, Debug, Deserialize)]
pub struct MMDBResult {
    #[serde(default)]
    pub asn: String,
    #[serde(default)]
    pub as_name: String,
    #[serde(default)]
    pub as_domain: String,
    #[serde(default)]
    pub continent: String,
    #[serde(default)]
    pub continent_name: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub country_name: String,
}
