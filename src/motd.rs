use rand::rng;
use rand::seq::IndexedRandom;

pub fn motd() -> String {
    let potentials = vec![
        "First we Ball, then we Atro. We Balatro!",
        "Someone call Bean!",
        "Surely you'll pull a baron this time.",
        "Also try the SealSeal mod!",
        "You are valid!",
        "Nerf Ceremonial Dagger.",
        "Praise Zech!",
        "Only cool people can see this message :3",
        ":3",
        "Also try Cloverpit!",
        "PhotoPhotoPhotoChadChad",
        // "You can add your own MOTD by opening a report at https://github.com/colonthreeing/balatro-tui!",
        //"Welcome to the balatui!",
        // he said I should keep this in but i'm going to be for real w you for a second i want this software to be more approachable
        // sorry
        // "Hey you should check out this guys mod, or wait, I dunno whatever you want to add, I kinda am paraphrasing here as my short term memory is imperfect but this is what I remember (This is in reference to what happened when I asked someone what I should add here)",
    ];

    let random = potentials.choose(&mut rng());
    random.unwrap().to_string()
}
