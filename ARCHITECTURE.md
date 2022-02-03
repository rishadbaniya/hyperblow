# Architecture

**Note :** This is my first time writing seriously how an application is architectured, so please fire a PR if you find anything wrong in writing the architecture.



**Hyperblow** uses two threads generally, one for rendering the UI(main thread) and other one for handling the torrent download part

For concurrency, i'm planning on using shared state concurrency to exchange data between threads. There are problems like handling the checkbox event and other stuff, so i might use message passing as well. For now, i'm not sure what i'm gonna use. 

Reference for the UDP Tracker Protocol :
http://www.bittorrent.org/beps/bep_0015.html
