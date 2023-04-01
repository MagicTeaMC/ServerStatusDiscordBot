FROM node:18
RUN mkdir statusbot
WORKDIR /statusbot
COPY package.json /statusbot/
RUN npm install
COPY . /statusbot/
RUN mv example.env .env
CMD [ "node", "index.js" ]
