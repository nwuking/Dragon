use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{stdin, stdout, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::u8;

static mut S_COMPLETION_CALLBACK: fn() = || {};
static mut S_HINTS_CALLBACK: fn() = || {};
static mut S_HISTORY_MAX_LEN: usize = 100;
static mut S_HISTORY: Option<VecDeque<String>> = None;

struct Linenoise2State<'a> {
    // TODO
    propmt: &'a str,
    buf: String,
    bufs: Vec<String>,
}

impl<'a> Linenoise2State<'a> {
    pub fn new(propmt: &'a str) -> Self {
        Self {
            propmt: propmt,
            buf: String::new(),
            bufs: Vec::new(),
        }
    }
}

fn linenoise2_edit_delete(l: &mut Linenoise2State) {
    match l.buf.pop() {
        Some(_) => {}
        None => {
            l.buf = match l.bufs.pop() {
                Some(buf) => buf,
                None => String::new(),
            };
        }
    }
}

fn linenoise2_edit_space(l: &mut Linenoise2State) {
    l.bufs.push(l.buf.clone());
    l.buf.clear();
}

fn linenoise2_edit_insert(l: &mut Linenoise2State, c: u8) {
    l.buf.push(char::from(c));
}

pub fn linenoise2_set_completion_callback(callback: fn()) {
    unsafe {
        S_COMPLETION_CALLBACK = callback;
        // S_COMPLETION_CALLBACK();
    }
}

pub fn linenoise2_set_hints_callback(callback: fn()) {
    unsafe {
        S_HINTS_CALLBACK = callback;
        // S_HINTS_CALLBACK();
    }
}

pub fn linenoise2_history_add(line: &str) {
    // 将历史记录添加到历史队列中
    unsafe {
        if S_HISTORY_MAX_LEN == 0 {
            return;
        }

        let history = match &mut S_HISTORY {
            Some(history) => history,
            None => {
                let history = VecDeque::new();
                S_HISTORY = Some(history);
                S_HISTORY.as_mut().unwrap()
            }
        };

        if history.len() == S_HISTORY_MAX_LEN {
            history.pop_front();
        }
        history.push_back(String::from(line));
    }
}

pub fn linenoise2_history_load(filename: &str) {
    // 打开文件
    let path = Path::new(filename);
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
    {
        Ok(file) => file,
        Err(e) => panic!("file: {} open error: {}", path.display(), e),
    };

    // 读取文件内容
    let buf_reader = BufReader::new(file);
    for line in buf_reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => panic!("file: {} read error: {}", path.display(), e),
        };
        linenoise2_history_add(line.as_str());
    }
}

pub fn linenoise2(propmt: &str) -> Option<Vec<String>> {
    let mut l = Linenoise2State::new(propmt);
    let mut result: Option<Vec<String>> = match stdout().write(l.propmt.as_bytes()) {
        Err(_) => None,
        _ => {
            // 刷新缓冲区
            stdout().flush().unwrap();
            Some(Vec::new())
        }
    };

    loop {
        // 一个字符一个字符的读取
        let mut buf: [u8; 1] = [0; 1];
        let n = match stdin().read(buf.as_mut()) {
            Err(_) => {
                result = None;
                break;
            }
            Ok(_) => buf[0],
        };

        let _ = match n {
            10 | 13 => {
                // enter
                linenoise2_edit_space(&mut l);
                result = Some(l.bufs);
                break;
            }
            3 => {
                // ctrl + c 退出程序
                // panic!("ctrl + c");
            }
            4 => {
                // ctrl + d
                // todo!("ctrl + d");
            }
            8 | 127 => {
                // backspace
                linenoise2_edit_delete(&mut l);
            }
            27 => {
                // esc
                // todo!("esc");
            }
            9 => {
                // tab
                // todo!("tab");
            }
            32 => {
                // space
                linenoise2_edit_space(&mut l);
            }
            _ => {
                // other
                linenoise2_edit_insert(&mut l, n);
            }
        };
    }

    result
}
