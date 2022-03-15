# 💻 🦀 rusty-boids 🐟 🐦
Using Rust and the Bevy game engine to make a simple boids simulation.

This is the first time I've messed around with the ECS architecture after watching a few cool talks about it.

This is also the first time in a while that I've touched Rust. It's all spaghetti code 🍝

Some Goals Right Now:
- Make the simulation faster through GPU compute ~~(DONE) parallel/concurrent code.~~
    - Or replace Bevy's Renderer as I don't need all that power. I can just keep the ECS.
- Add killing between the creature groups.
- Add more parameters to tweak around.
- Add ability to mate and reproduce.