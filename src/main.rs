use ftp::FtpStream;
use reqwest::blocking::Client;
use reqwest::header;
use reqwest::header::COOKIE;
use serde_json::Value;
use std::{error::Error, io};

struct Token {
    client_id: String,
    uid: String,
}

impl Token {
    fn build(client_id: String, uid: String) -> Token {
        Token { client_id, uid }
    }
}

fn ftp_login(ip: &str, username: &str, password: &str) -> FtpStream {
    let mut ftp_stream = FtpStream::connect(ip).unwrap();
    ftp_stream.login(username, password).unwrap();

    println!("登录成功！\n");

    ftp_stream
}

fn select_homework(ftp_stream: &mut FtpStream, dir_list: Vec<String>) {
    println!("当前目录下的文件夹有：");
    for i in dir_list.iter() {
        println!("{}", i);
    }
    println!("");

    println!("请输入要提交的作业编号：");
    let mut idx = String::new();
    io::stdin().read_line(&mut idx).unwrap();

    let idx: usize = idx.trim().parse().unwrap();
    if idx >= dir_list.len() {
        panic!("不合法的编号");
    }

    ftp_stream.cwd(&dir_list[idx]).unwrap();

    println!("您当前所在的文件夹为 {}\n", ftp_stream.pwd().unwrap());
}

fn get_token() -> Token {
    println!("请输入 __client_id：");
    let mut client_id = String::new();
    io::stdin().read_line(&mut client_id).unwrap();
    client_id = String::from(client_id.trim());
    client_id = "__client_id=".to_string() + &client_id;

    println!("请输入 _uid：");
    let mut uid = String::new();
    io::stdin().read_line(&mut uid).unwrap();
    uid = String::from(uid.trim());
    uid = "_uid=".to_string() + &uid;

    Token::build(client_id, uid)
}

fn get_client(token: &Token) -> Result<Client, Box<dyn Error>> {
    let cookie = token.client_id.clone() + &";".to_string() + &token.uid;

    let mut map = header::HeaderMap::new();
    map.insert(
        "x-luogu-type",
        header::HeaderValue::from_static("content-only"),
    );
    map.insert(COOKIE, header::HeaderValue::from_str(&cookie)?);

    let client = reqwest::blocking::Client::builder()
        .default_headers(map)
        .build()?;
    Ok(client)
}

fn get_problem_list(client: &Client) -> Result<Vec<String>, Box<dyn Error>> {
    println!("请输入洛谷题单编号：");
    let mut id = String::new();
    io::stdin().read_line(&mut id).unwrap();
    let id: usize = id.trim().parse().unwrap();

    let url = "https://www.luogu.com.cn/training/".to_string() + &id.to_string();
    let raw_str = client.get(url).send()?.text()?;
    let json: Value = serde_json::from_str(&raw_str)?;

    let list = &json["currentData"]["training"]["problems"];
    let mut res: Vec<String> = Vec::new();
    if let Value::Array(list) = list {
        for i in list.iter() {
            let i = &i["problem"]["pid"];
            if let Value::String(i) = i {
                res.push(i.to_string());
            }
        }
    }

    Ok(res)
}

fn get_problem_status(
    pid: &String,
    uid: &String,
    client: &Client,
) -> Result<String, Box<dyn Error>> {
    let url = "https://www.luogu.com.cn/record/list";
    let params = [("pid", pid), ("user", uid), ("status", &"12".to_string())];
    let raw_str = client.get(url).query(&params).send()?.text()?;
    let json: Value = serde_json::from_str(&raw_str)?;

    Ok(json["currentData"]["records"]["result"][0]["id"].to_string())
}

fn get_code_by_rid(rid: String, client: &Client) -> Result<String, Box<dyn Error>> {
    let url = "https://www.luogu.com.cn/record/".to_string() + &rid;
    let raw_str = client.get(url).send()?.text()?;
    let json: Value = serde_json::from_str(&raw_str)?;

    let str = &json["currentData"]["record"]["sourceCode"];

    match str {
        Value::String(str) => Ok(str.to_string()),
        _ => Err(String::from("failed to get code").into()),
    }
}

fn ftp_put(pid: &String, code: &mut String, ftp_stream: &mut FtpStream) -> io::Result<()> {
    let file_name = pid.clone() + &".cpp".to_string();
    ftp_stream.put(&file_name, &mut code.as_bytes()).unwrap();
    Ok(())
}

fn main() {
    let mut username = String::new();
    let mut password = String::new();
    println!("请输入帐号：");
    io::stdin().read_line(&mut username).unwrap();
    username = String::from(username.trim());
    println!("请输入密码：");
    io::stdin().read_line(&mut password).unwrap();
    password = String::from(password.trim());

    let mut ftp_stream = ftp_login("192.168.50.175:21", &username, &password);

    let list = ftp_stream.list(None).unwrap();

    let mut dir_list: Vec<String> = Vec::new();
    for name in list.iter() {
        if &name[0..1] == "d" {
            let mut name_iter = name.trim().split_whitespace();
            let name = name_iter.next_back().unwrap();

            if let Some(_) = name.get(0..1) {
                dir_list.push(String::from(name));
            }
        }
    }

    select_homework(&mut ftp_stream, dir_list);

    let token = get_token();
    let client = get_client(&token).unwrap();

    let uid = token.uid.split('=').next_back().unwrap().to_string();

    let problem_list = get_problem_list(&client).unwrap();
    for pid in problem_list.iter() {
        let str = get_problem_status(&pid, &uid, &client).unwrap();
        if str != "null" {
            let mut code = get_code_by_rid(str, &client).unwrap();
            ftp_put(&pid, &mut code, &mut ftp_stream).unwrap();
        }
    }

    println!("传输成功，感谢使用。")
}
