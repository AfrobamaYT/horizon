fn entry(time: i64) {
    if time % 100 == 0 {
        fnr_loop::note(&format!("HEAD tick_block_entities at game time {time}"));
    }
}
