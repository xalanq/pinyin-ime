extern crate pinyin_ime;
/*
use pinyin_ime::collections as col;
use pinyin_ime::gen;
use pinyin_ime::load::*;
*/
use pinyin_ime::eval::PinyinIME;
use std::env;

fn main() {
    /*
    let (hanzi_v, hanzi_m) = load_hanzi("./data/hanzi.txt");
    gen::gen_word("./tmp/word", "./tmp/word/dict.utf8", "./data/word.txt", &hanzi_m, 7);
    gen::gen_sen("./tmp/raw/sina", "./tmp/text/sina/sina.txt", &hanzi_m, col::gen_sina);
    gen::gen_sen("./tmp/raw/student", "./tmp/text/student/student.txt", &hanzi_m, col::gen_raw);
    gen::gen_sen("./tmp/raw/test", "./tmp/text/test/sina.txt", &hanzi_m, col::gen_sina);

    let (word_v, word_m, pinyin_v, _) = load_word("./data/word.txt", &hanzi_m);
    gen::gen_jieba_dict("./data/jieba.txt", &hanzi_v, &word_v);
    let jb = load_jieba("./data/jieba.txt");
    gen::gen_dev(
        "./tmp/text/test/sina.txt",
        "./tmp/text/test/input.txt",
        "./tmp/text/test/answer.txt",
        &hanzi_m,
        &word_m,
        &pinyin_v,
        &jb,
    );

    gen::gen_gram(
        "./tmp/text/sina",
        "./data/sina",
        (20000000, 20000000, 22000000),
        &hanzi_m,
        &word_m,
        &jb,
        (2, 2, 2),
    );
    gen::gen_gram(
        "./tmp/text/student",
        "./data/student",
        (20000000, 20000000, 20000000),
        &hanzi_m,
        &word_m,
        &jb,
        (1, 1, 1),
    );
    gen::gen_total_gram("./data");
    */
    /*
    for c in ["char", "word"].iter() {
        let data_path = &format!("./data/{}", c);
        for i in 1..=9 {
            let config_path = &format!("./data/cfg/{}_{}.json", c, i);
            let output_path = &format!("./tmp/text/test/{}_output_{}.json", c, i);
            let ime = PinyinIME::new(config_path, data_path);
            ime.eval_from("./tmp/text/test/input.txt", "./tmp/text/test/answer.txt", output_path);
        }
    }
    */
    let ime = PinyinIME::new("./data/config.json", "./data");
    match env::args().len() {
        3 => {
            let input = env::args().nth(1).unwrap();
            let output = env::args().nth(2).unwrap();
            ime.eval_from_only(&input, &output);
        }
        4 => {
            let input = env::args().nth(1).unwrap();
            let output = env::args().nth(2).unwrap();
            let answer = env::args().nth(3).unwrap();
            ime.eval_from(&input, &answer, &output);
        }
        /*
        2 => {
            let k = env::args()
                .nth(1)
                .unwrap()
                .parse::<usize>()
                .expect("你需要输入一个整数，表示输出的答案个数");
            if k == 0 {
                println!("参数必须大于0");
            } else {
                println!("将会显示 {} 个解以及概率", k);
                println!("请尽情输入拼音: ");
                let mut input = String::new();
                while let Ok(_) = std::io::stdin().read_line(&mut input) {
                    if input.len() == 0 {
                        break;
                    }
                    let preds = ime.evals(&input, k);
                    if preds.len() == 0 {
                        println!("Error: 无法识别");
                    } else {
                        print!(
                            "{}",
                            preds.iter().fold(&mut String::new(), |s, t| {
                                if s.len() > 0 {
                                    s.push_str(&format!("{} {}\n", t.0, t.1));
                                } else {
                                    s.push_str(&format!("{} {}\n", t.0, t.1));
                                }
                                s
                            })
                        );
                    }
                    input.clear();
                }
                println!("Exit. Good luck!");
            }
        }
        */
        _ => {
            println!("请尽情输入拼音: ");
            let mut input = String::new();
            while let Ok(_) = std::io::stdin().read_line(&mut input) {
                if input.len() == 0 {
                    break;
                }
                match ime.eval(&input) {
                    Some(ans) => println!("{}", ans.0),
                    None => println!("Error: 无法识别"),
                };
                input.clear();
            }
            println!("Exit. Good luck!");
        }
    };
}
