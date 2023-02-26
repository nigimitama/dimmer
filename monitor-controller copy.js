const { PythonShell } = require("python-shell");
let pyshell = new PythonShell("python_modules/control_monitors.py");

exports.getLuminance = () => {
  // sends a message to the Python script via stdin
  pyshell.send("get");

  var response = "";
  pyshell.on("message", function (message) {
    // received a message sent from the Python script (a simple "print" statement)
    response = message;
    console.log(`message is ${message}`);
  });
  console.log(`response is ${response}`);

  // end the input stream and allow the process to exit
  pyshell.end(function (err, code, signal) {
    if (err) throw err;
    console.log("The exit code was: " + code);
    console.log("The exit signal was: " + signal);
    console.log("finished");
  });
  console.log(`response is ${response}`);
  return response;
};

exports.setLuminance = (value) => {
  // sends a message to the Python script via stdin
  pyshell.send("set");
  pyshell.send(`${value}`);

  pyshell.on("message", function (message) {
    // received a message sent from the Python script (a simple "print" statement)
    console.log(message);
  });

  // end the input stream and allow the process to exit
  pyshell.end(function (err, code, signal) {
    if (err) throw err;
    console.log("The exit code was: " + code);
    console.log("The exit signal was: " + signal);
    console.log("finished");
  });
};
