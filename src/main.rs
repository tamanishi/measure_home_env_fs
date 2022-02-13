extern crate bme280;
extern crate google_firestore1_beta1 as firestore1_beta1;
extern crate hyper;
extern crate hyper_native_tls;
extern crate linux_embedded_hal as hal;
extern crate yup_oauth2 as oauth2;
extern crate clap;

use bme280::BME280;
use chrono::Local;
use firestore1_beta1::Document;
use firestore1_beta1::Error;
use firestore1_beta1::Firestore;
use firestore1_beta1::Value;
use hal::{Delay, I2cdev};
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use std::collections::HashMap;
use std::default::Default;
use std::path::Path;
use yup_oauth2::GetToken;
use clap::{App, Arg};
use std::fmt;

const BME280_DEVICE: &str = "/dev/i2c-1";
const FS_CREDENTIAL_FILE: &str = "home-env-firebase-adminsdk.json";
const FS_API_SCOPE_DATASTORE: &str = "https://www.googleapis.com/auth/datastore";
const FS_DOCUMENT_PATH: &str = "projects/home-env/databases/(default)/documents";
const FS_COLLECTION_NAME: &str = "measurements";

struct MeasurementDoc {
    datetime: Value,
    temperature: Value,
    humidity: Value,
    pressure: Value,
}

impl MeasurementDoc {
    fn to_string_value(raw: String) -> Value {
        let mut val = Value::default();
        val.string_value = Some(raw);
        val
    }
    fn to_double_value(raw: f32) -> Value {
        let mut val = Value::default();
        val.double_value = Some(raw as f64);
        val
    }
    pub fn new(datetime: String, temperature: f32, humidity: f32, pressure: f32) -> MeasurementDoc {
        MeasurementDoc {
            datetime: MeasurementDoc::to_string_value(datetime),
            temperature: MeasurementDoc::to_double_value(temperature),
            humidity: MeasurementDoc::to_double_value(humidity),
            pressure: MeasurementDoc::to_double_value(pressure / 100.0),
        }
    }
    pub fn to_hashmap(self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("datetime".to_string(), self.datetime);
        map.insert("temperature".to_string(), self.temperature);
        map.insert("humidity".to_string(), self.humidity);
        map.insert("pressure".to_string(), self.pressure);
        map
    }
}

impl fmt::Display for MeasurementDoc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"\"datetime\": {:?}, \"temperature\": {:.3}, \"humidity\": {:.3}, \"pressure\": {:.3}", self.datetime.string_value.as_ref().unwrap(), self.temperature.double_value.unwrap(), self.humidity.double_value.unwrap(), self.pressure.double_value.unwrap())
    }
}

fn main() {
    let app = App::new("measure_home_env_fs")
        .version("0.2.0")
        .author("Masayuki Sunahara <tamanishi.gmail.com>")
        .about("measures home temperature, humidity, and pressure. ")
        .arg(Arg::with_name("dryrun")
            .help("Shows on console only")
            .long("dryrun")
            .short("d")
    );

    let matches = app.get_matches();

    let i2c_bus = I2cdev::new(BME280_DEVICE).unwrap();

    let mut bme280 = BME280::new_primary(i2c_bus, Delay);

    bme280.init().unwrap();

    let measurements = bme280.measure().unwrap();

    let now = Local::now();
    let now_str = now.clone().format("%Y/%m/%d %H:%M:%S").to_string();
    let now_id = now.clone().format("%Y%m%d%H%M%S").to_string();

    let measurement_doc = MeasurementDoc::new(
        now_str,
        measurements.temperature,
        measurements.humidity,
        measurements.pressure,
    );

    if matches.is_present("dryrun") {
//        println!("dryrun!!");
        println!("{{{}}}", measurement_doc);
        return;
    }
 
    let exe_file_path = std::env::current_exe().unwrap();
    let exe_dir_path = exe_file_path.parent().unwrap();
    let cred_dir_path = match Path::new("/.dockerenv").exists() {
        true => "/cred",
        false => {
            exe_dir_path.to_str().unwrap()
        },
    };
    let client_secret = oauth2::service_account_key_from_file(
        &format!("{}/{}", cred_dir_path, FS_CREDENTIAL_FILE).to_string(),
    )
    .unwrap();

    let client =
        hyper::Client::with_connector(HttpsConnector::new(NativeTlsClient::new().unwrap()));
    let mut access = oauth2::ServiceAccountAccess::new(client_secret, client);

    access.token(&vec![FS_API_SCOPE_DATASTORE]).unwrap();

    let client =
        hyper::Client::with_connector(HttpsConnector::new(NativeTlsClient::new().unwrap()));
    let hub = Firestore::new(client, access);

    let map = MeasurementDoc::to_hashmap(measurement_doc);

    let mut req = Document::default();
    req.fields = Some(map);

    let result = hub
        .projects()
        .databases_documents_create_document(req, FS_DOCUMENT_PATH, FS_COLLECTION_NAME)
        .document_id(&now_id)
        .doit();

    match result {
        Err(e) => match e {
            Error::HttpError(_)
            | Error::MissingAPIKey
            | Error::MissingToken(_)
            | Error::Cancelled
            | Error::UploadSizeLimitExceeded(_, _)
            | Error::Failure(_)
            | Error::BadRequest(_)
            | Error::FieldClash(_)
            | Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok(_) => {}
    }
}
