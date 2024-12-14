use reqwest::Client;
use serde_json::{self, Value};
use sha256::digest;
use std::collections::HashMap;
use std::fs::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread::sleep;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use uuid::*;
mod common;

fn main() {
    let lang_json_path = load_lang_path(Path::new("zh.json").to_path_buf());
    let lang_json_data = read_lang_file(&lang_json_path);
    let json_value = serde_json::Value::from_str(lang_json_data.as_str()).unwrap();
    let lang = "en"; //当前需要翻译的语言 de,en,es,fr,it,ja,ko,pt,ru
    let save_as = format!("{lang}.lang.json");
    let json_translation = build_translation_map(json_value, lang);
    out_lang_text_target(&json_translation, Path::new(&save_as).to_path_buf());
    println!("{:?}", json_translation);
}

async fn run(query: &str, to: &str) -> Result<Value, Box<dyn std::error::Error>> {
    sleep(std::time::Duration::from_millis(1000));
    let salt = Uuid::new_v4().to_string();
    let app_id = "app_id";
    let cur_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let sign_input = format!(
        "{}{}{}{}{}",
        app_id,
        query_truncate(query),
        &salt,
        &cur_time,
        "app_key"
    );
    let sign = digest(sign_input).to_string();
    let data: HashMap<String, String> = {
        let mut data = HashMap::new();
        data.insert("q".to_string(), query.to_string());
        data.insert("from".to_string(), "zh-CHS".to_string());
        data.insert("to".to_string(), to.to_string());
        data.insert("appKey".to_string(), app_id.to_string());
        data.insert("sign".to_string(), sign);
        data.insert("salt".to_string(), salt);
        data.insert("signType".to_string(), "v3".to_string());
        data.insert("curtime".to_string(), cur_time);
        data
    };
    let response = Client::new()
        .get("https://openapi.youdao.com/api")
        .query(&data)
        .send()
        .await?
        .text()
        .await?;
    let json_data = response.as_str();
    let person = serde_json::from_str::<Value>(json_data).unwrap();
    let lang = match person["translation"].clone() {
        Value::Array(trans) => trans[0].clone(),
        _ => panic!("{:?},{query},{to}", person),
    };
    Ok(lang)
}

fn load_lang_path(path: PathBuf) -> std::path::PathBuf {
    let dir_name = common::path::__dirname();
    let dir_path = std::path::Path::new(&dir_name);
    let lang_path = dir_path.join("resource").join(path);
    lang_path
}

fn read_lang_file(lang_path: &std::path::PathBuf) -> String {
    std::fs::read_to_string(lang_path).unwrap()
}

fn build_translation_map(json_value: Value, to: &str) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    if let Value::Object(obj) = &json_value {
        for (key, value) in obj.iter() {
            match value {
                Value::Object(nest_obj) => {
                    let nest_map = build_translation_map(Value::Object(nest_obj.clone()), to);
                    map.insert(key.to_string(), Value::Object(nest_map));
                }
                Value::String(value) => {
                    let runtime = Runtime::new().unwrap();
                    runtime.block_on(async {
                        let lang = run(value.as_str(), to).await.unwrap();
                        println!("{key},{value} => {lang}");
                        map.insert(key.clone(), lang);
                    });
                }
                _ => (),
            }
        }
    }
    map
}

/** 翻译接口签名 query 长度大于20时 input = q 前10个字符串 + q 长度 + q 后 10个 */
fn query_truncate(query: &str) -> String {
    let input = query.chars();
    let l = input.clone().count();
    if l <= 20 {
        String::from(query)
    } else {
        let pre: String = input.clone().take(10).collect();
        let last: String = input.clone().skip(l - 10).take(10).collect();
        format!("{pre}{l}{last}")
    }
}

fn out_lang_text_target(obj: &serde_json::Map<String, Value>, target: PathBuf) {
    let dir_name = common::path::__dirname();
    let output = dir_name.join("output").join(target);
    let json_obj = serde_json::to_string(&obj).unwrap();
    let mut lang_file = File::create(output).unwrap();
    let result = lang_file.write(json_obj.as_bytes());
    match result {
        Ok(_) => println!("写入翻译文件成功!!!"),
        Err(error) => panic!("{error}"),
    };
}
