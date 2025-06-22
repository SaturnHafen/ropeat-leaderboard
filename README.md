# Leaderboard for the [_Ropeat_](https://github.com/forUnity/Ropeat/) Game

## Submitting scores

```sh
curl --request POST --json '{"score": 1337, "color": "#123456"}' --header 'Authorization: abcd' http://localhost:3000/backend/submit_score
```

## Claiming scores

1. head to [http://localhost:3000/claim/list](http://localhost:3000/claim/list)
2. click on the score you want to claim
3. fill out the form
4. if you checked the "möchtest du auf dem Leaderboard auftauchen" checkbox, your score will be shown [here](http://localhost:3000/)
