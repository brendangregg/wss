#!/usr/bin/env ruby

require 'time'
require 'rmagick'
require 'gruff'

if ARGV.length != 1
  puts("Usage: create_gif.rb pid")
  exit
end
pid = ARGV[0]

Dir.chdir("/tmp/wss/#{pid}")
timestamps = Dir.glob("20*").sort
puts("Using timestamps #{timestamps}")
virtual_addresses = Dir.glob("#{timestamps[-2]}/0x*").map{|f| f.split("/").last}
if virtual_addresses.length != 1
  puts("Got invalid number of virtual addresses: #{virtual_addresses}")
end
virtual_address = virtual_addresses.first

# This will get messy if we add in page content hashes...
image_size = Math.sqrt(File.size("#{timestamps[-2]}/#{virtual_address}")).ceil()
puts("Using image size #{image_size} from file size #{File.size("#{timestamps[-2]}/#{virtual_address}")}")

label_border = (image_size / 30.0).ceil()

gif = Magick::ImageList.new

active_pages_arr = Array.new
zero_pages_arr = Array.new
mapped_pages_arr = Array.new

pixels = Array.new(3 * image_size * image_size, 0)
(0...timestamps.size).each do |timestamp_idx|
  puts("#{timestamp_idx} / #{timestamps.size}")
  file = File.open("#{timestamps[timestamp_idx]}/#{virtual_address}")
  active_pages = 0
  zero_pages = 0
  mapped_pages = 0
  page_idx = 0
  file.each_byte do |byte|
    activity_mask = 0x03
    zero_mask = 1 << 2
    zero_add = ((byte & zero_mask) == 0) ? 0 : 0x8000
    zero_pages += 1 if zero_add != 0
    if ((byte & activity_mask) == 0)
      pixels[(3 * page_idx) + 0 ] = 0
      pixels[(3 * page_idx) + 1 ] = 0
      pixels[(3 * page_idx) + 2 ] = 0
    elsif ((byte & activity_mask) == 0x02)
      pixels[(3 * page_idx) + 0 ] = 0
      pixels[(3 * page_idx) + 1 ] = 0x7FFF + zero_add
      pixels[(3 * page_idx) + 2 ] = 0
      mapped_pages += 1
    elsif ((byte & activity_mask) == 0x03)
      pixels[(3 * page_idx) + 0 ] = 0x7FFF + zero_add
      pixels[(3 * page_idx) + 1 ] = 0
      pixels[(3 * page_idx) + 2 ] = 0
      active_pages += 1
      mapped_pages += 1
    else
      puts("Unknown byte: #{byte & mask}")
      exit
    end
    page_idx += 1
  end

  active_pages_arr << active_pages / page_idx.to_f
  zero_pages_arr << zero_pages / page_idx.to_f
  mapped_pages_arr << mapped_pages / page_idx.to_f

  image = Magick::Image.new(image_size, image_size) { self.background_color = "black" }
  puts("Image size: #{image_size}")
  puts("Pixel size: #{pixels.length}")
  image.import_pixels(0, 0, image_size, image_size, "RGB", pixels)
  image = image.extent(image_size, image_size + label_border)
  desc_txt = Magick::Draw.new
  image.annotate(desc_txt, 0, 0, 0, 0, "SPECjbb 1 core, 3100 MiB RAM, 2500 MiB heap") {
    desc_txt.pointsize = label_border * 0.9
    desc_txt.fill = 'red'
    desc_txt.gravity = Magick::SouthWestGravity
    desc_txt.font_weight = Magick::BoldWeight
  }
  frame_txt = Magick::Draw.new
  delta_t = Time.parse(timestamps[timestamp_idx]) - Time.parse(timestamps.first)
  image.annotate(frame_txt, 0, 0, 0, 0, Time.at(delta_t).utc.strftime("%H:%M:%S")) {
    frame_txt.pointsize = label_border * 0.9
    frame_txt.fill = 'white'
    frame_txt.gravity = Magick::SouthEastGravity
    frame_txt.font_weight = Magick::BoldWeight
  }
  #image.colorspace = Magick::GRAYColorspace
  gif << image
  image.write("img/#{timestamp_idx.to_s.rjust(3, "0")}.png")
end
gif.delay = 100
gif.write("img/gif.gif")

graph = Gruff::Line.new
graph.data("mapped", mapped_pages_arr)
graph.data("zeros", zero_pages_arr)
graph.data("active", active_pages_arr)
graph.write("img/graph.png")
