sayHi("SETUP")
console2.log("SETUP")

addEventListener("random", (e) => {
  console2.log("wow, it's " + e.value);
})


function tick() {
  // demo()
  // sayHi("Rucola")
  // console2.log("2x3=" + multiply(2, 3))
  // console2.log("2x0=" + multiply(2, 0))
  // console2.log(returnObject())
}


// TODO: hide and insert automatically
function loop() {
  handle_events()
  tick()
  setTimeout(loop, 100);
}

loop()
