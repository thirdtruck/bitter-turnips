# 04May2020

Better late than never! Wishing I'd started this from minute one of the game jam but there's still plenty to talk about.

`slotmap` provides support for multiple key types, in order to support stronger type checks between its maps. I'll try adding a `VillagerKey` and see how well that works.

Never mind! All the maps I have right now actually need the same key. I'll revisit this later, like if/when I had a reverse look-up table for coordinates.

Let's try making inline event generation more functional (as opposed to mutable) instead.

That wasn't too bad! At least not the groundwork of refactoring the event resolution methods to return new events instead of modifying the new event stack in place.