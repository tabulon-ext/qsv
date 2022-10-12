#!/bin/bash

# This script does some very basic benchmarks with 'qsv' using a 520mb, 41 column, 1M row 
# sample of NYC's 311 data. If it doesn't exist on your system, it will be downloaded for you.
#
# These aren't meant to be overly rigorous, but they should be enough to catch
# significant regressions.
#
# Make sure you're using a release-optimized `qsv - generated by 
# `cargo build --release`; `cargo install qsv`; or `cargo install --path .` 
# issued from the root of your qsv git repo.
#
# This shell script has been tested on Linux, macOS and Cygwin for Windows.
# It requires 7-Zip (https://www.7-zip.org/download.html) as we need the high compression ratio
# so we don't have to deal with git-lfs to host the large compressed file on GitHub.
# On Cygwin, you also need to install `bc` and `time`.

set -e

pat="$1"
# you can change bin_name to another binary, like xsv
bin_name=qsv
datazip=/tmp/NYC_311_SR_2010-2020-sample-1M.7z
data=NYC_311_SR_2010-2020-sample-1M.csv
commboarddata=communityboards.csv
data_idx=NYC_311_SR_2010-2020-sample-1M.csv.idx
data_to_exclude=data_to_exclude.csv
searchset_patterns=searchset_patterns.txt
if [ ! -r "$data" ]; then
  printf "Downloading benchmarking data...\n"
  curl -sS https://raw.githubusercontent.com/wiki/jqnatividad/qsv/files/NYC_311_SR_2010-2020-sample-1M.7z > "$datazip"
  7z e -y "$datazip"
  "$bin_name" sample --seed 42 1000 "$data" -o "$data_to_exclude"
  printf "homeless\npark\nnoise\n" > "$searchset_patterns"
  curl -sS https://raw.githubusercontent.com/wiki/jqnatividad/qsv/files/communityboards.csv > "$commboarddata"
fi
os_type=$(echo $OSTYPE | cut -c 1-6)
if [[ "$os_type" == "darwin" ]]; then
  data_size=$(stat -f '%z' "$data")
else
  data_size=$(stat --format '%s' "$data")
fi

function real_seconds {
  cmd=$(echo $@ "> /dev/null 2>&1")
  t=$(
    $(which time) -p sh -c "$cmd" 2>&1 \
      | grep '^real' \
      | awk '{print $2}')
  if [ $(echo "$t < 0.01" | bc) = 1 ]; then
    t=0.01
  fi
  echo $t
}

function benchmark {
  rm -f "$data_idx"
  t1=$(real_seconds "$@")
  rm -f "$data_idx"
  t2=$(real_seconds "$@")
  rm -f "$data_idx"
  t3=$(real_seconds "$@")
  echo "scale=2; ($t1 + $t2 + $t3) / 3" | bc
}

function benchmark_with_index {
  rm -f "$data_idx"
  "$bin_name" index "$data"
  t1=$(real_seconds "$@")
  t2=$(real_seconds "$@")
  t3=$(real_seconds "$@")
  rm -f "$data_idx"
  echo "scale=2; ($t1 + $t2 + $t3) / 3" | bc
}

function run {
  index=
  while true; do
    case "$1" in
      --index) index="yes" && shift ;;
      *) break ;;
    esac
  done
  name="$1"
  shift

  printf "%-27s" "$name"
  if [ -z "$pat" ] || echo "$name" | grep -E -q "^$pat$"; then
    if [ -z "$index" ]; then
      t=$(benchmark "$@")
    else
      t=$(benchmark_with_index "$@")
    fi
    mb_per=$(echo "scale=2; ($data_size / $t) / 2^20" | bc)
    recs_per=$(echo "scale=2; (1000000 / $t)" | bc)
    mb_per=$(printf "%0.02f" $mb_per)
    recs_per=$(printf "%'.2f" $recs_per)
    printf -v tprint "%0.02f" $t
    printf "%-11s%-12s%-12s\n" "$tprint" "$mb_per" "$recs_per"
    printf "%s\t%0.02f\t%s\t%s\n" $name $t $mb_per $recs_per >> $benchmarkfile
  fi
}

binver1=$("$bin_name" --version)
binver=$(echo ${binver1:4:6})
current_time=$(date "+%Y-%m-%d-%H-%M-%S")
benchmarkfile=$bin_name-bench-$binver-$current_time.tsv
printf "%-27s%-11s%-12s%-12s\n" BENCHMARK TIME_SECS MB_PER_SEC RECS_PER_SEC
printf "benchmark\ttime_secs\tmb_per_sec\trecs_per_sec\n" > $benchmarkfile
run apply_op_string "$bin_name" apply operations lower Agency -q "$data"
run apply_op_similarity "$bin_name" apply operations lower,simdln Agency --comparand brooklyn --new-column Agency_sim-brooklyn_score -q "$data"
run apply_op_soundex "$bin_name" apply operations lower,soundex Agency --comparand Queens --new-column Agency_queens_soundex -q "$data" 
run apply_datefmt "$bin_name" apply datefmt \"Created Date\" -q "$data"
run apply_emptyreplace "$bin_name" apply emptyreplace \"Bridge Highway Name\" --replacement Unspecified -q "$data"
run apply_geocode "$bin_name" apply geocode Location --new-column geocoded_location -q "$data"
run count "$bin_name" count "$data"
run --index count_index "$bin_name" count "$data"
run dedup "$bin_name" dedup "$data"
run enum "$bin_name" enum "$data"
run exclude "$bin_name" exclude 'Incident Zip' "$data" 'Incident Zip' "$data_to_exclude"
run --index exclude_index "$bin_name" exclude 'Incident Zip' "$data" 'Incident Zip' "$data_to_exclude"
run explode "$bin_name" explode City "-" "$data"
run fill "$bin_name" fill -v Unspecified 'Address Type' "$data"
run fixlengths "$bin_name" fixlengths "$data"
run flatten "$bin_name" flatten "$data"
run flatten_condensed "$bin_name" flatten "$data" --condense 50
run fmt "$bin_name" fmt --crlf "$data"
run frequency "$bin_name" frequency "$data"
run --index frequency_index "$bin_name" frequency "$data"
run frequency_selregex "$bin_name" frequency -s /^R/ "$data"
run frequency_j1 "$bin_name" frequency -j 1 "$data"
run index "$bin_name" index "$data"
run join "$bin_name" join --no-case 'Community Board' "$data" community_board "$commboarddata"
run lua "$bin_name" lua map location_empty "tonumber\(Location\)==nil" -q "$data"
run partition "$bin_name" partition 'Community Board' /tmp/partitioned "$data"
run pseudo "$bin_name" pseudo 'Unique Key' "$data"
run rename "$bin_name" rename 'unique_key,created_date,closed_date,agency,agency_name,complaint_type,descriptor,loctype,zip,addr1,street,xstreet1,xstreet2,inter1,inter2,addrtype,city,landmark,facility_type,status,due_date,res_desc,res_act_date,comm_board,bbl,boro,xcoord,ycoord,opendata_type,parkname,parkboro,vehtype,taxi_boro,taxi_loc,bridge_hwy_name,bridge_hwy_dir,ramp,bridge_hwy_seg,lat,long,loc' "$data"
run replace "$bin_name" replace '\b[Nn]\.*[Yy]\.*[Cc]\.*\b' 'New York City' "$data"
run replace_unicode "$bin_name" replace '\b[Nn]\.*[Yy]\.*[Cc]\.*\b' 'New York City' --unicode "$data"
run reverse "$bin_name" reverse "$data"
run sample_10 "$bin_name" sample 10 "$data"
run --index sample_10_index "$bin_name" sample 10 "$data"
run sample_1000 "$bin_name" sample 1000 "$data"
run --index sample_1000_index "$bin_name" sample 1000 "$data"
run sample_100000 "$bin_name" sample 100000 "$data"
run --index sample_100000_index "$bin_name" sample 100000 "$data"
run sample_100000_seeded "$bin_name" sample 100000 --seed 42 "$data"
run --index sample_100000_seeded_index "$bin_name" sample --seed 42 100000 "$data"
run --index sample_25pct_index "$bin_name" sample 0.25 "$data"
run --index sample_25pct_seeded_index "$bin_name" sample 0.25 --seed 42 "$data"
run search "$bin_name" search -s 'Agency Name' "'(?i)us'" "$data"
run search_unicode "$bin_name" search --unicode -s 'Agency Name' "'(?i)us'" "$data"
run searchset "$bin_name" searchset "$searchset_patterns" "$data"
run searchset_unicode "$bin_name" searchset "$searchset_patterns" --unicode "$data"
run select "$bin_name" select 'Agency,Community Board' "$data"
run select_regex "$bin_name" select /^L/ "$data"
run slice_one_middle "$bin_name" slice -i 500000 "$data"
run --index slice_one_middle_index "$bin_name" slice -i 500000 "$data"
run sort "$bin_name" sort -s 'Incident Zip' "$data"
run sort_random_seeded "$bin_name" sort --random --seed 42 "$data"
run split "$bin_name" split --size 50000 split_tempdir "$data"
run --index split_index "$bin_name" split --size 50000 split_tempdir "$data"
run --index split_index_j1 "$bin_name" split --size 50000 -j 1 split_tempdir "$data"
run stats "$bin_name" stats "$data"
run --index stats_index "$bin_name" stats "$data"
run --index stats_index_j1 "$bin_name" stats -j 1 "$data"
run stats_everything "$bin_name" stats "$data" --everything
run stats_everything_j1 "$bin_name" stats "$data" --everything -j 1
run --index stats_everything_index "$bin_name" stats "$data" --everything
run --index stats_everything_index_j1 "$bin_name" stats "$data" --everything -j 1
run table "$bin_name" table "$data"
run transpose "$bin_name" transpose "$data"
