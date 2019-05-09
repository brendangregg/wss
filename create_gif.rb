#!/usr/bin/env ruby

require 'rmagick'

pid = "10092"
virtual_address = "0x7fd607823000"

Dir.chdir("/tmp/wss/#{pid}")
timestamps = Dir.glob("155*").sort
puts("Using timestamps #{timestamps}")

image_size = Math.sqrt(File.size("#{timestamps[-2]}/#{virtual_address}") * 4).ceil()
puts("Using image size #{image_size} from file size #{File.size("#{timestamps[-2]}/#{virtual_address}")}")

gif = Magick::ImageList.new

pixels = Array.new(3 * image_size * image_size, 0)
masks = (0...4).map { |shift| 0x3 << (2 * shift) }
puts(masks)
(0...timestamps.size).each do |timestamp_idx|
  puts("#{timestamp_idx} / #{timestamps.size}")
  file = File.open("#{timestamps[timestamp_idx]}/#{virtual_address}")

  byte_cnt = 0
  file.each_byte do |byte|
    masks.each_with_index do |mask, mask_idx|
      page_idx = (byte_cnt * 4) + mask_idx
      if ((byte & mask) == 0)
        pixels[(3 * page_idx) + 0 ] = 0
        pixels[(3 * page_idx) + 1 ] = 0
        pixels[(3 * page_idx) + 2 ] = 0
      elsif ((byte & mask) == (0xAA & mask))
        pixels[(3 * page_idx) + 0 ] = 0
        pixels[(3 * page_idx) + 1 ] = 0x7FFF
        pixels[(3 * page_idx) + 2 ] = 0
      elsif ((byte & mask) == (0xFF & mask))
        pixels[(3 * page_idx) + 0 ] = 0x7FFF
        pixels[(3 * page_idx) + 1 ] = 0
        pixels[(3 * page_idx) + 2 ] = 0
      else
        puts("Unknown byte: #{byte & mask}")
        exit
      end
    end
    byte_cnt += 1
  end
  image = Magick::Image.new(image_size, image_size) { self.background_color = "red" }
  puts("Image size: #{image_size}")
  puts("Pixel size: #{pixels.length}")
  image.import_pixels(0, 0, image_size, image_size, "RGB", pixels)
  desc_txt = Magick::Draw.new
  image.annotate(desc_txt, 0, 0, 0, 0, "SPECjbb 2 core, 16 GiB") {
    desc_txt.pointsize = 50
    desc_txt.fill = 'red'
    desc_txt.gravity = Magick::SouthWestGravity
    desc_txt.font_weight = Magick::BoldWeight
  }
  frame_txt = Magick::Draw.new
  delta_t = timestamps[timestamp_idx].to_i - timestamps.first.to_i
  image.annotate(frame_txt, 0, 0, 0, 0, Time.at(delta_t).utc.strftime("%H:%M:%S")) {
    frame_txt.pointsize = 50
    frame_txt.fill = 'red'
    frame_txt.gravity = Magick::SouthEastGravity
    frame_txt.font_weight = Magick::BoldWeight
  }
  #image.colorspace = Magick::GRAYColorspace
  gif << image
  image.write("img/#{timestamp_idx.to_s.rjust(3, "0")}.png")
end
gif.delay = 100
gif.write("img/gif.gif")
