use memorize_func::{Lazy, LruCache};
use std::sync::Mutex;

static FIBO_MAP: Lazy<Mutex<LruCache<usize, u32>>> =
    Lazy::new(|| Mutex::new(LruCache::new(100.try_into().unwrap())));

fn fibo_original(n: usize) -> u32 {
    if n == 0 {
        return 0;
    } else if n == 1 {
        return 1;
    }

    let n_m1_value = fibo_original(n - 1);
    let n_m2_value = fibo_original(n - 2);

    n_m1_value + n_m2_value
}

fn fibo_cached(n: usize) -> u32 {
    // キャッシュの中にあったらそれを返す
    {
        if let Some(value) = FIBO_MAP.lock().unwrap().get(&n) {
            return value.clone();
        }
    }

    if n == 0 {
        return 0;
    } else if n == 1 {
        return 1;
    }

    let n_m1_value = fibo_cached(n - 1);
    let n_m2_value = fibo_cached(n - 2);

    let answer = n_m1_value + n_m2_value;

    // キャッシュに追加
    {
        FIBO_MAP.lock().unwrap().push(n, answer.clone());
    }

    answer
}

fn main() {
    use std::time::Instant;

    let mut ret: u32 = 0;
    let start_time = Instant::now();

    for _ in 0..10 {
        ret = fibo_original(35);
    }
    println!("{ret:?}");
    println!("original: {:?}", Instant::now() - start_time);

    let start_time = Instant::now();
    for _ in 0..10 {
        ret = fibo_cached(35);
    }
    println!("{ret:?}");
    println!("cached: {:?}", Instant::now() - start_time);
}
