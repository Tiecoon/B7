use std::fmt::Debug;

/// returns average of slice
fn get_average(input: &[i64]) -> i64 {
    if input.is_empty() {
        panic!("Divide by zero!");
    }
    input.iter().fold(0, |mut sum, i| {
        sum += i;
        sum
    }) / (input.len() as i64)
}

/// find the largest outlier in given slice
pub fn find_outlier<I: Debug>(counts: &[(I, i64)]) -> &(I, i64) {
    let second: Vec<i64> = counts.iter().map(|i| i.1).collect();
    let avg: i64 = get_average(&second[..]);

    // and then find the most distant point
    let mut max_dist: i64 = -1;
    let mut max_idx: usize = 0;
    for (i, (_, count)) in counts.iter().enumerate() {
        let dist: i64 = (count - avg).abs();
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    &counts[max_idx]
}

#[cfg(test)]
mod tests {
    use super::{find_outlier, get_average};
    use std::str::FromStr;

    #[test]
    fn test_average() {
        let pairs: &[(&[i64], i64)] = &[(&[0], 0), (&[1], 1), (&[0, 1], 0), (&[1, 3], 2)];
        for i in pairs {
            assert!(get_average(i.0) == i.1);
        }
    }

    #[test]
    #[should_panic]
    fn test_average_panic() {
        get_average(&[] as &[i64]);
    }

    #[test]
    fn outlier_test() {
        let pairs = &[(
            &[
                (String::from_str("a"), 0),
                (String::from_str("b"), 0),
                (String::from_str("c"), 1),
            ],
            &(String::from_str("c"), 1),
        )];
        for i in pairs {
            assert!(find_outlier(i.0) == i.1);
        }
    }

    #[test]
    #[should_panic]
    fn outlier_test_panic() {
        find_outlier(&[] as &[(String, i64)]);
    }
}
