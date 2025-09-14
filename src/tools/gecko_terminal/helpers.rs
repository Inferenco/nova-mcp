pub(crate) fn build_url(base: &str, segments: &[&str]) -> String {
    let mut url = base.trim_end_matches('/').to_string();
    for segment in segments {
        url.push('/');
        url.push_str(segment.trim_matches('/'));
    }
    url
}
