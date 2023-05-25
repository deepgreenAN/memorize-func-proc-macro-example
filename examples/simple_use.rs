use memorize_func::memorize_func;

#[memorize_func(size = 100)]
fn fibo(n: usize) -> u32 {
    if n == 0 {
        return 0;
    } else if n == 1 {
        return 1;
    }

    let n_m1_value = fibo(n - 1);
    let n_m2_value = fibo(n - 2);

    n_m1_value + n_m2_value
}

fn main() {
    use std::time::Instant;

    let start_time = Instant::now();
    let mut ret: u32 = 0;

    for _ in 0..10 {
        ret = fibo(35);
    }
    println!("{ret:?}");
    println!("cached: {:?}", Instant::now() - start_time);
}
