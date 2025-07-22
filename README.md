# Leaderboard for the [_Ropeat_](https://github.com/forUnity/Ropeat/) Game

## Submitting scores

```sh
curl --request POST --json '{"score": 1337, "color": "#123456"}' --header 'Authorization: abcd' http://localhost:3000/backend/submit_score
```

The score format is pretty simple: You submit a color as a hashtag with six hex-digits and a score as a positive integer. Currently the ordering of the scores is the higher the better. The input will be validated pretty strictly (see `backend/src/lib.rs#submit_score`).

- color validation regex: [`#[0-9a-fA-F]{6}`](https://regexper.com/#%23%5B0-9a-fA-F%5D%7B6%7D)
- score validation: positive 32 bit integer (range: `0 - 2_147_483_647`)

## Claiming scores

1. head to [http://localhost:3000/claim/list](http://localhost:3000/claim/list)
2. click on the score you want to claim
3. fill out the form
4. if you checked the "möchtest du auf dem Leaderboard auftauchen" checkbox, your score will be shown [here](http://localhost:3000/)
