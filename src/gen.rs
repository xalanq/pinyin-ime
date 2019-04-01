use crate::load::{hanzi2word, word2hanzi};
use jieba_rs::Jieba;
use pbr::ProgressBar;
use pinyin;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::time::Duration;

fn gen_gram_filter(
    s: &str,
    gram_1: &mut HashMap<usize, usize>,
    gram_2: &mut HashMap<(usize, usize), usize>,
    gram_3: &mut HashMap<(usize, usize, usize), usize>,
    gram_4: &mut HashMap<(usize, usize, usize, usize), usize>,
    hanzi_m: &HashMap<char, usize>,
    word_m: &HashMap<Vec<usize>, usize>,
    jb: &Jieba,
) {
    jb.cut(&s, false).split(|c| *c == " ").for_each(|ws| {
        let sen: Vec<_> = ws.iter().map(|w| hanzi2word(*w, hanzi_m, word_m)).collect();
        // start
        if sen.len() > 0 && sen[0] != 1 {
            *gram_2.entry((0, sen[0])).or_insert(0) += 1;
            if sen.len() > 1 && sen[1] != 1 {
                *gram_3.entry((0, sen[0], sen[1])).or_insert(0) += 1;
                if sen.len() > 2 && sen[2] != 1 {
                    *gram_4.entry((0, sen[0], sen[1], sen[2])).or_insert(0) += 1;
                }
            }
        }
        for i in (0..sen.len()).rev() {
            let x = sen[i];
            if x == 1 {
                continue; // unknown
            }
            *gram_1.entry(x).or_insert(0) += 1;
            if i + 1 < sen.len() {
                let y = sen[i + 1];
                if y == 1 {
                    continue; // unknown
                }
                *gram_2.entry((x, y)).or_insert(0) += 1;
                if i + 2 < sen.len() {
                    let z = sen[i + 2];
                    if z == 1 {
                        continue; // unknown
                    }
                    *gram_3.entry((x, y, z)).or_insert(0) += 1;
                    if i + 3 < sen.len() {
                        let t = sen[i + 3];
                        if t == 1 {
                            continue; // unknown
                        }
                        *gram_4.entry((x, y, z, t)).or_insert(0) += 1;
                    }
                }
            }
        }
    });
}

fn gen_gram_one(
    path: &str,
    gram_1: &mut HashMap<usize, usize>,
    gram_2: &mut HashMap<(usize, usize), usize>,
    gram_3: &mut HashMap<(usize, usize, usize), usize>,
    gram_4: &mut HashMap<(usize, usize, usize, usize), usize>,
    hanzi_m: &HashMap<char, usize>,
    word_m: &HashMap<Vec<usize>, usize>,
    jb: &Jieba,
) {
    println!("...Working on {}", path);
    let data = fs::read_to_string(path).expect(&format!("......Cannot read {}", path));
    let mut iter = data.lines();
    let mut pb = ProgressBar::new(iter.next().unwrap().parse().unwrap());
    pb.set_max_refresh_rate(Some(Duration::from_secs(1)));
    let mut gc = 0;
    macro_rules! gc {
        () => {
            gram_1.retain(|_, v| *v >= 3);
            gram_2.retain(|_, v| *v >= 3);
            gram_3.retain(|_, v| *v >= 3);
            gram_4.retain(|_, v| *v >= 3);
        };
    }
    for line in iter {
        gen_gram_filter(line, gram_1, gram_2, gram_3, gram_4, hanzi_m, word_m, jb);
        pb.add(1);
        gc += 1;
        if gc % 30000 == 0 {
            gc!();
        }
    }
    gc!();
    pb.finish_println("");
    println!(
        "......len of gram 1: {}\n......len of gram 2: {}\n......len of gram 3: {}\n\
         ......len of gram 4: {}",
        gram_1.len(),
        gram_2.len(),
        gram_3.len(),
        gram_4.len(),
    );
}

pub fn gen_gram(
    path: &str,
    save_path: &str,
    k_th: (usize, usize, usize, usize),
    hanzi_v: &Vec<char>,
    hanzi_m: &HashMap<char, usize>,
    word_v: &Vec<Vec<usize>>,
    word_m: &HashMap<Vec<usize>, usize>,
    jb: &Jieba,
) {
    println!("Generating gram-n");
    let paths = fs::read_dir(path).unwrap();
    let mut gram_1 = HashMap::new();
    let mut gram_2 = HashMap::new();
    let mut gram_3 = HashMap::new();
    let mut gram_4 = HashMap::new();
    for path in paths {
        let path = path.unwrap();
        if path.metadata().unwrap().is_file() {
            gen_gram_one(
                &path.path().display().to_string(),
                &mut gram_1,
                &mut gram_2,
                &mut gram_3,
                &mut gram_4,
                hanzi_m,
                word_m,
                jb,
            );
        }
    }
    println!("...summaring");
    let mut gram_1: Vec<_> = gram_1.into_iter().collect();
    let mut gram_2: Vec<_> = gram_2.into_iter().collect();
    let mut gram_3: Vec<_> = gram_3.into_iter().collect();
    let mut gram_4: Vec<_> = gram_4.into_iter().collect();
    let k1 = gram_1.len().min(k_th.0);
    let k2 = gram_2.len().min(k_th.1);
    let k3 = gram_3.len().min(k_th.2);
    let k4 = gram_4.len().min(k_th.3);
    kth(&mut gram_1, k1);
    kth(&mut gram_2, k2);
    kth(&mut gram_3, k3);
    kth(&mut gram_4, k4);
    gram_1.truncate(k1);
    gram_2.truncate(k2);
    gram_3.truncate(k3);
    gram_4.truncate(k4);
    gram_1.sort_unstable_by(|&(a, b), &(c, d)| {
        if word_v[a].len() == word_v[c].len() {
            d.cmp(&b)
        } else {
            word_v[a].len().cmp(&word_v[c].len())
        }
    });
    gram_2.sort_unstable_by(|&(a, b), &(c, d)| {
        if word_v[a.0].len() == word_v[c.0].len() {
            if word_v[a.1].len() == word_v[c.1].len() {
                d.cmp(&b)
            } else {
                word_v[a.1].len().cmp(&word_v[c.1].len())
            }
        } else {
            word_v[a.0].len().cmp(&word_v[c.0].len())
        }
    });
    gram_3.sort_unstable_by(|&(a, b), &(c, d)| {
        if word_v[a.0].len() == word_v[c.0].len() {
            if word_v[a.1].len() == word_v[c.1].len() {
                if word_v[a.2].len() == word_v[c.2].len() {
                    d.cmp(&b)
                } else {
                    word_v[a.2].len().cmp(&word_v[c.2].len())
                }
            } else {
                word_v[a.1].len().cmp(&word_v[c.1].len())
            }
        } else {
            word_v[a.0].len().cmp(&word_v[c.0].len())
        }
    });
    gram_4.sort_unstable_by(|&(a, b), &(c, d)| {
        if word_v[a.0].len() == word_v[c.0].len() {
            if word_v[a.1].len() == word_v[c.1].len() {
                if word_v[a.2].len() == word_v[c.2].len() {
                    if word_v[a.3].len() == word_v[c.3].len() {
                        d.cmp(&b)
                    } else {
                        word_v[a.3].len().cmp(&word_v[c.3].len())
                    }
                } else {
                    word_v[a.2].len().cmp(&word_v[c.2].len())
                }
            } else {
                word_v[a.1].len().cmp(&word_v[c.1].len())
            }
        } else {
            word_v[a.0].len().cmp(&word_v[c.0].len())
        }
    });
    println!(
        "......len of gram 1: {}\n......len of gram 2: {}\n......len of gram 3: {}\n\
         ......len of gram 4: {}",
        gram_1.len(),
        gram_2.len(),
        gram_3.len(),
        gram_4.len(),
    );
    println!("...writing");
    let errmsg = &format!("......Cannot save to {}", save_path);
    File::create(&format!("{}{}", save_path, "/gram_1.txt"))
        .expect(errmsg)
        .write_all(
            gram_1
                .iter()
                .fold(&mut String::new(), |s, &(a, b)| {
                    s.push_str(&format!("{} {}\n", word2hanzi(a, hanzi_v, word_v), b));
                    s
                })
                .as_bytes(),
        )
        .expect(errmsg);
    File::create(&format!("{}{}", save_path, "/gram_2.txt"))
        .expect(errmsg)
        .write_all(
            gram_2
                .iter()
                .fold(&mut String::new(), |s, &(a, b)| {
                    s.push_str(&format!(
                        "{} {} {}\n",
                        word2hanzi(a.0, hanzi_v, word_v),
                        word2hanzi(a.1, hanzi_v, word_v),
                        b
                    ));
                    s
                })
                .as_bytes(),
        )
        .expect(errmsg);
    File::create(&format!("{}{}", save_path, "/gram_3.txt"))
        .expect(errmsg)
        .write_all(
            gram_3
                .iter()
                .fold(&mut String::new(), |s, &(a, b)| {
                    s.push_str(&format!(
                        "{} {} {} {}\n",
                        word2hanzi(a.0, hanzi_v, word_v),
                        word2hanzi(a.1, hanzi_v, word_v),
                        word2hanzi(a.2, hanzi_v, word_v),
                        b
                    ));
                    s
                })
                .as_bytes(),
        )
        .expect(errmsg);
    File::create(&format!("{}{}", save_path, "/gram_4.txt"))
        .expect(errmsg)
        .write_all(
            gram_4
                .iter()
                .fold(&mut String::new(), |s, &(a, b)| {
                    s.push_str(&format!(
                        "{} {} {} {} {}\n",
                        word2hanzi(a.0, hanzi_v, word_v),
                        word2hanzi(a.1, hanzi_v, word_v),
                        word2hanzi(a.2, hanzi_v, word_v),
                        word2hanzi(a.3, hanzi_v, word_v),
                        b
                    ));
                    s
                })
                .as_bytes(),
        )
        .expect(errmsg);
    println!("...Done!");
}

pub fn gen_total_gram(path: &str) {
    println!("Generating total gram-n");
    let config = &format!("{}/config.json", path);
    let data = fs::read_to_string(config).expect(&format!("...Unable to read {}", config));
    let data = serde_json::from_str::<Value>(&data).expect("...Cannot convert to json");
    let data = data.as_object().expect("...Invalid json");
    let mut sum = 0.0;
    let mut total_gram = Vec::new();
    for _ in 0..4 {
        total_gram.push(HashMap::new());
    }
    data.iter().for_each(|(k, v)| {
        let w = v.as_f64().expect("...Invalid number");
        let mut gram = Vec::new();
        for _ in 0..4 {
            gram.push(Vec::new());
        }
        for i in 0..4 {
            let fname = &format!("{}/{}/gram_{}.txt", path, k, i + 1);
            println!("...Working on {}", fname);
            let mut tot = 0;
            fs::read_to_string(fname)
                .expect(&format!("......Cannot read {}", fname))
                .lines()
                .for_each(|line| {
                    let p = line.rfind(" ").unwrap();
                    let num = line[p + 1..].parse::<usize>().unwrap();
                    tot += num;
                    gram[i].push((line[0..p].to_string(), num));
                });
            gram[i].iter().for_each(|(a, b)| {
                *total_gram[i].entry(a.clone()).or_insert(0.0) += *b as f64 * w / tot as f64;
            });
        }
        sum += w;
    });
    let mut gram: Vec<Vec<(String, f64)>> = Vec::new();
    for i in 0..4 {
        total_gram[i].iter_mut().for_each(|(_, b)| *b = *b * 10000.0 / sum);
        let mut tmp = Vec::new();
        total_gram[i].iter().for_each(|(k, v)| tmp.push((k.clone(), *v)));
        tmp.sort_unstable_by(
            |(a, b), (c, d)| {
                if a == c {
                    d.partial_cmp(b).unwrap()
                } else {
                    a.cmp(c)
                }
            },
        );
        gram.push(tmp);
    }
    for i in 0..4 {
        let fname = &format!("{}/gram_{}.txt", path, i + 1);
        let errmsg = &format!("...Cannot save to {}", fname);
        File::create(fname)
            .expect(errmsg)
            .write_all(
                gram[i]
                    .iter()
                    .fold(&mut String::new(), |s, (a, b)| {
                        s.push_str(&format!("{} {}\n", a, b));
                        s
                    })
                    .as_bytes(),
            )
            .expect(errmsg);
    }
    println!("...Done!");
}

fn word_filter(s: &str, data: &mut HashSet<String>, hanzi_m: &HashMap<char, usize>) {
    s.split_whitespace().for_each(|a| {
        let mut valid = true;
        let mut len = 0;
        for c in a.chars() {
            if hanzi_m.get(&c).is_none() {
                valid = false;
                break;
            }
            len += 1;
        }
        if valid && len > 1 && len <= 7 {
            data.insert(a.to_string());
        }
    });
}

fn gen_word_one(path: &str, data: &mut HashSet<String>, hanzi_m: &HashMap<char, usize>) {
    println!("...Working on {}", path);
    word_filter(
        &fs::read_to_string(path).expect(&format!("......Cannot read {}", path)),
        data,
        hanzi_m,
    );
    println!("......total len: {}", data.len());
}

pub fn gen_word(path: &str, ref_path: &str, save_path: &str, hanzi_m: &HashMap<char, usize>) {
    println!("Generating word");
    let paths = fs::read_dir(path).unwrap();
    let mut data = HashSet::new();
    hanzi_m.iter().for_each(|c| {
        data.insert(c.0.to_string());
    });
    for path in paths {
        let path = path.unwrap();
        if path.metadata().unwrap().is_file() {
            gen_word_one(&path.path().display().to_string(), &mut data, hanzi_m);
        }
    }
    let mut data: Vec<_> = data.into_iter().collect();
    data.sort_by(|a, b| if a.len() != b.len() { a.len().cmp(&b.len()) } else { a.cmp(b) });

    let mut ref_py = HashMap::new();
    fs::read_to_string(ref_path).expect(&format!("...Cannot read {}", ref_path)).lines().for_each(
        |line| {
            let data: Vec<_> = line.split_whitespace().collect();
            let mut valid = true;
            for c in data[0].chars() {
                if hanzi_m.get(&c).is_none() {
                    valid = false;
                    break;
                }
            }
            if valid {
                data[2..].iter().for_each(|s| {
                    ref_py
                        .entry(data[0].to_string())
                        .or_insert(Vec::new())
                        .push(s.split(':').next().unwrap().to_string())
                });
            }
        },
    );

    let mut args = pinyin::Args::new();
    args.heteronym = true;
    let errmsg = &format!("...Cannot save to {}", save_path);
    let mut pb = ProgressBar::new(data.len() as u64);
    pb.set_max_refresh_rate(Some(Duration::from_secs(1)));
    File::create(save_path)
        .expect(errmsg)
        .write_all(
            data.iter()
                .fold(&mut String::new(), |s, a| {
                    match ref_py.get(a) {
                        Some(p) => s.push_str(&format!("{} {}\n", a, p.join(" "))),
                        None => s.push_str(&format!(
                            "{} {}\n",
                            a,
                            pinyin::lazy_pinyin(a, &args).join("'")
                        )),
                    }
                    pb.add(1);
                    s
                })
                .as_bytes(),
        )
        .expect(errmsg);
    pb.finish_println("");
    println!("...Done!");
}

pub fn gen_jieba_dict(save_path: &str, hanzi_v: &Vec<char>, word_v: &Vec<Vec<usize>>) {
    println!("Generating jieba dictionary");
    let jb = Jieba::new();
    let errmsg = &format!("...Cannot save to {}", save_path);
    File::create(save_path)
        .expect(errmsg)
        .write_all(
            (2..word_v.len())
                .fold(&mut String::new(), |s, i| {
                    let w = word2hanzi(i, hanzi_v, word_v);
                    let freq = jb.suggest_freq(&w);
                    s.push_str(&format!("{} {}\n", w, freq));
                    s
                })
                .as_bytes(),
        )
        .expect(errmsg);
    println!("...Done!");
}

fn kth<T>(a: &mut Vec<(T, usize)>, k: usize) {
    if a.len() == 0 {
        return;
    }
    let (mut l, mut r) = (0, a.len() - 1);
    let k = k - 1;
    while l <= r {
        let mut pos = l;
        for i in l..r {
            if a[i].1 > a[r].1 {
                a.swap(i, pos);
                pos += 1;
            }
        }
        a.swap(pos, r);
        if pos == k {
            return;
        } else if pos > k {
            r = pos - 1;
        } else {
            l = pos + 1;
        }
    }
}
