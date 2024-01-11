# First, run sudo docker run -it -v ${HOME}/output:/app/src-tauri/target/release tauri-app
# Then, while inside container terminal, run npm run tauri build
# Finally, copy the AppImage to your local files


p/src-tauri/target/release tauri-app
# Use an official Ubuntu runtime as the base image
FROM ubuntu:latest

# Update the system
RUN apt-get update && apt-get upgrade -y

# Install necessary dependencies
RUN apt-get install -y curl
RUN curl -fsSL https://deb.nodesource.com/setup_21.x | bash -
RUN apt-get install nodejs -y


RUN apt-get install -y git rustc cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --target armv7-unknown-linux-gnueabihf
RUN apt-get install --assume-yes --no-install-recommends pkg-config openssl webkit2gtk-4.0 javascriptcoregtk-4.0 libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libatk1.0-dev libgdk-pixbuf-2.0-dev libcairo2-dev libpango1.0-dev libgtk-3-dev libsoup2.4-dev libssl-dev wget patchelf file

# Set the working directory
WORKDIR /app

# Copy your project into the Docker container
COPY . .

# Install Node.js dependencies
RUN npm install

# Install Rust dependencies
# RUN cargo build --release -v

# Build your Tauri app
RUN npm run tauri build

# Copy the resulting AppImage from the Docker container to your host machine
VOLUME /app/src-tauri/target/release
