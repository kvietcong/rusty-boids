# 💻 🦀 rusty-boids 🐟 🐦

![rusty-boids](https://github.com/user-attachments/assets/87900683-8353-4c9a-a9f5-2c25ba085401)

Using Rust and the Bevy game engine to make a simple boids simulation. See live demo site here: <https://kvietcong.github.io/rusty-boids/>

This is the first time I've messed around with the ECS architecture after watching a few cool talks about it.

This is also the first time in a while that I've touched Rust. It's all spaghetti code 🍝

Some Goals Right Now:
- Make the simulation faster through GPU compute ~~(DONE) parallel/concurrent code.~~
    - Or replace Bevy's Renderer as I don't need all that power. I can just keep the ECS.
- Add killing between the creature groups.
- Add more parameters to tweak around.
