# Bird Of The Day
A Bluesky bot. This is the code used to generate a bird of the day for the Bluesky account: [`@birdofthedaybot`](https://bsky.app/profile/birdofthedaybot.bsky.social).

All bird information comes from [eBird.org](https://ebird.org).

## How it works
1. A local copy of the eBird bird database is retrieved through [their API](https://documenter.getpostman.com/view/664302/S1ENwy59#952a4310-536d-4ad1-8f3e-77cfb624d1bc).
2. The bird database, in addition to specific birds, also contains individual bird species (e.g., *Apteryx sp.*). Additionally, the database contains extinct birds (e.g., Dodo). Neither of these are desirable for posting, so they are filtered before posting.
3. After filtering out species and extinct birds, a random bird is selected from the remaining birds.
4. The selected bird is then obtained from [eBird.org](eBird.org) which allows downloading the characteristic bird image.
5. Finally, after obtaining a random bird and its picture, the [Bluesky API](https://docs.bsky.app/), is used to upload the image and make the final post.