<h1>LANChat - a simple chat inside your local network</h1>
<h2>About</h2>
<div>
  LANChat is a very simple and minimalistic chat program that runs in your terminal. It is written in Rust and is cross-platform compatible (I think, I only tested it on Linux :P)
</div>
<h2>How to use</h2>
<p>1.) Clone this GitHub repository with git:</p>

```
git clone https://github.com/TAugustL/LANChat.git
```

<p>2.) Enter the created folder.</p>
<p>3.) If you are the host of the chat enter:</p>

```
cargo run --release server (optional: port)
```

<p>You will see your IP address and the port the server is listening to (the person you are chatting with will need this)</p>
<p>4.) If you are the client (not the host) you need to enter:</p>

```
cargo run --release client [IP address of the host and port, e.g. 192.168.2.1:8888]
```

<p>Now both devices should enter the chat room. Type your messages and press enter to send them!</p>
![game_demo](https://github.com/TAugustL/LANChat/blob/main/preview.png)
