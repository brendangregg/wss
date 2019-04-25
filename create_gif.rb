#!/usr/bin/env ruby

require 'rmagick'

pid = "10514"
virtual_address = "0x7f3552da2000"

Dir.chdir("/tmp/wss/#{pid}")
timestamps = Dir.glob("155*").sort
puts("Using timestamps #{timestamps}")

image_size = Math.sqrt(File.size("#{timestamps[-2]}/#{virtual_address}") * 8).ceil()
puts("Using image size #{image_size} from file size #{File.size("#{timestamps[-2]}/#{virtual_address}")}")


pixels = Array.new(image_size * image_size, 0)
masks = (0...8).map { |shift| 1 << shift }
(0...timestamps.size).each do |timestamp_idx|
	puts("#{timestamp_idx} / #{timestamps.size}")
	file = File.open("#{timestamps[timestamp_idx]}/#{virtual_address}")

	byte_cnt = 0
	file.each_byte do |byte|
		masks.each_with_index do |mask, mask_idx|
			page_idx = (byte_cnt * 8) + mask_idx
			if ((byte & mask) != 0)
				pixels[page_idx] = 0xFFFF
			else
				pixels[page_idx] = 0
			end
		end
		byte_cnt += 1
	end
	image = Magick::Image.new(image_size, image_size) { self.background_color = "black" }
	image.colorspace = Magick::GRAYColorspace
	image.import_pixels(0, 0, image_size, image_size, "I", pixels)
	image.write("img/#{timestamp_idx.to_s.rjust(3, "0")}.jpg")
end
