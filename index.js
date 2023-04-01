console.clear()
const Discord = require("discord.js")
const os = require("os")
const si = require("systeminformation")
const client = new Discord.Client({
  intents: ["Guilds","GuildMessages","MessageContent"],
})
require("dotenv").config()

///////// Config /////////
const token = process.env.token
const channelID = process.env.channelID

let statusMessageID

client.on("ready", async () => {
  console.log("Bot is ready!")
  const channel = client.channels.cache.get(channelID)
  const messages = await channel.messages.fetch()
  messages.forEach((msg) => {
    if (msg.author == client.user.id) return statusMessageID = msg.id
    msg.delete()
  })
  setInterval(async () => {
    const cpuUsage = Math.round(process.cpuUsage().system / 1000000)
    const memUsage = await si.mem().then((data) => {
      const totalMemory = Math.round(data.total / 1024 / 1024)
      const freeMemory = Math.round(data.free / 1024 / 1024)
      const usedMemory = totalMemory - freeMemory
      const usedMemoryPercentage = Math.round((usedMemory / totalMemory) * 100)
      return `${usedMemory}MB/${totalMemory}MB (${usedMemoryPercentage}%)`
    })
    const disks = await si.fsSize().then((data) => {
      let diskUsage = ""
      data.forEach((disk) => {
        const totalDisk = Math.round(disk.size / 1024 / 1024)
        const freeDisk = Math.round(disk.available / 1024 / 1024)
        const usedDisk = totalDisk - freeDisk
        const usedDiskPercentage = Math.round((usedDisk / totalDisk) * 100)
        diskUsage += `${disk.mount}\n${usedDisk}MB/${totalDisk}MB (${usedDiskPercentage}%)\n`
      })
      return diskUsage
    })
    const netUsage = await si.networkStats().then((data) => {
      const bytesIn = data[0].rx_sec
      const bytesOut = data[0].tx_sec
      const kbytesIn = bytesIn / 1024
      const kbytesOut = bytesOut / 1024
      return `In: ${kbytesIn.toFixed(2)} KB/s Out: ${kbytesOut.toFixed(2)} KB/s`
    })
    const statusMessage = `CPU Usage: ${cpuUsage}%\nMemory Usage: ${memUsage}\nDisk Usage:\n${disks}Network Usage: ${netUsage}\nLast Update: <t:${Math.floor(Date.now()/1000)}:R>`

    if (statusMessageID) {
      const statusChannel = client.channels.cache.get(channelID)
      statusChannel.messages.fetch(statusMessageID).then((message) => {
        message.edit(statusMessage)
      })
    } else {
      const statusChannel = client.channels.cache.get(channelID)
      statusChannel.send(statusMessage).then((message) => {
        statusMessageID = message.id
      })
    }
    console.log("Update!")
  }, 5000)
})

client.login(token)
process.on("uncaughtException",console.log)