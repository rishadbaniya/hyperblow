# Architecture

**Note :** This is my first time writing seriously how an application is architectured, so please fire a PR if you find anything wrong in writing the architecture.

## Introduction

**Hyperblow** uses two threads generally, one for rendering the UI(main thread) and other one for handling the torrent download part

## Concurrency

For concurrency, i'm planning on using shared state concurrency to exchange data between threads. There are problems like handling the checkbox event and other stuff, so i might use message passing as well. For now, i'm not sure what i'm gonna use.

## Technical References

- Reference for the **UDP Tracker Protocol.**[link 1](http://xbtt.sourceforge.net/udp_tracker_protocol.html) / [link 2](http://www.bittorrent.org/beps/bep_0015.html)
- [Reference for the Peer ID field used in "Announce"](https://www.bittorrent.org/beps/bep_0020.html)
- [Extensions for Partial Seeds](http://www.bittorrent.org/beps/bep_0021.html)
- [Fast Extension](http://www.bittorrent.org/beps/bep_0006.html)
- [Extension Protocol](http://www.bittorrent.org/beps/bep_0010.html)

## How UDP request response works

I've used this mechanism, where i run two futures concurrently. First one is a future that polls all the trackers i.e it sends Connect Request, Announce Request and Scrape Request, other is the future that listens to the UDP socket and as soon as it gets some message on the UDP socket, it looks at the Socket Address that message came from sends the message back to the tracker for which the message came using  ```channel::Sender```


## Connecting to **peers** and getting pieces(blocks) (through TCP)

In order to connect to peers and start sending and receiving pieces, first of all we must make a TCP connection with the peer. After making TCP connection with the peer, we send something called a "Handshake" message and receive a "Handshake" response.

## Message Flow

Once you have received peer's ip address, you can use it to send a "Handshake" **Message**. The tricky part comes right here, we expect one of 11 Message Type to be sent by peer as a response to that Handshake Message, but what happens is that sometimes there is some sort of incosistency. It means, when we can recieve multiple Message in single packet i.e we can end up getting a very long unusual message consisting of several *Message* at the same time. Eg both **Handshake** and **Bitfield** in the same packet. We need to build some sort of mechanism to deal with this inconsistency of *Message*

A client can send us a series of Have messages, one for each piece it has. Alternatively, at the start of a connection, the peer can send a ‘Bitfield’ message. Bitfield messages are optional and can only be sent as the message immediately following the handshake message.


# Engine

It's the core abstraction of overall torrent session, it handles everything and acts as an backend for frontends, may it be CLI or GUI. It provides various API to access different torrent sessions, controlling the engine.

A basic possible Engine Architecture : 
![image](https://user-images.githubusercontent.com/54678051/216830912-81e0a44e-7fce-4700-97af-9db4b89b61df.png)\
![image](https://user-images.githubusercontent.com/54678051/216831243-e854c244-d39e-4662-be2d-91154fa0926f.png)


