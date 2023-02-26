const { PythonShell } = require("python-shell");

exports.getLuminance = async () => {
  // return: array[string]
  const result = await PythonShell.run("python_modules/get_luminance.py")
  return result
};

exports.setLuminance = async (value = null) => {
  const options = {
    args: [value]
  }
  return await PythonShell.run("python_modules/set_luminance.py", options)
};
