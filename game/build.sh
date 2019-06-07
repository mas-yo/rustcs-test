#!/bin/sh

dotnet publish --self-contained -r linux-x64 -c Release game.csproj -o bin

