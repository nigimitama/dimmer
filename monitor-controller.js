const { PythonShell } = require("python-shell");

exports.getLuminance = async () => {
  // return type is array[string]
  const result = await PythonShell.run("python_modules/get_luminance.py")
  console.log(result)
  return result
};

exports.setLuminance = async (value) => {
  const options = {
    args: [value]
  }
  return await PythonShell.run("python_modules/set_luminance.py", options)
};
