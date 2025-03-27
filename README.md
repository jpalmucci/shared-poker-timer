# Shared Poker Timer

It's annoying when you are playing a home poker tournament and you have to yell down the hall to the other table when the levels go up. 

It's also annoying when the person who is holding the clock busts out and goes home.

This is a little project that I wrote to:
1) fix these problems
2) learn Rust programming (because I think it's cool)

The current version of the timer is hosted at https://pokertimer.palmucci.net/. Feel free to use it for your own games if you want. It's running on a $4/month Digital Ocean server, which should be good for hundreds if not thousands of running timers.

## Usage

You can create a new poker timer on the home page. Once you do, simply share the link with people to whom you want to give access. The QR code is a simple way to share the link when you are sitting down to play.

### Notifications

In order to get notifications on an iPhone or iPad, you need to add the timer to your home screen. Click on the share icon and select "Add to Home Screen." When you turn on notifications, it is only for the currently running tournament. You just click the checkbox when a new tournament starts to start getting notifications.

### Structures

As of now, there is no structure editor. If you want to add a new structure, create an issue on Github (or better yet, a pull request).